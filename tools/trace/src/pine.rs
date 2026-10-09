//! Client for PCSX2's PINE IPC server (pcsx2/PINE.cpp, v2.8.2): read EE memory from a running
//! game without a savestate. Enable it in PCSX2: Settings > Advanced > "Enable PINE" (slot 28011).
//!
//! Transport on macOS/Linux: Unix socket `$TMPDIR/pcsx2.sock` (macOS; `$XDG_RUNTIME_DIR` on
//! Linux, `/tmp` fallback), with `.<slot>` appended when the slot is not 28011.
//! Message: `u32 total_len` then commands back to back (`u8 opcode, args`). Reply: `u32 total_len,
//! u8 status (0 ok, 0xff fail)`, then each command's result in order. Limits: request < 650000
//! bytes, reply < 450000 bytes, so reads are batched as up to 50000 `MsgRead64` per message.

use anyhow::{bail, ensure, Context, Result};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

pub const DEFAULT_SLOT: u16 = 28011;
const MSG_READ64: u8 = 3;
const MSG_WRITE8: u8 = 4;
const MSG_VERSION: u8 = 8;
const MSG_SAVE_STATE: u8 = 9;
const MSG_TITLE: u8 = 0xb;
const MSG_ID: u8 = 0xc;
const MSG_STATUS: u8 = 0xf;
const READS_PER_MSG: usize = 50_000;

pub struct Pine { s: UnixStream }

pub fn socket_path(slot: u16) -> PathBuf {
    let dir = std::env::var_os(if cfg!(target_os = "macos") { "TMPDIR" } else { "XDG_RUNTIME_DIR" }).map(PathBuf::from).unwrap_or_else(|| "/tmp".into());
    let mut name = "pcsx2.sock".to_string();
    if slot != DEFAULT_SLOT { name += &format!(".{slot}"); }
    dir.join(name)
}

impl Pine {
    pub fn connect(slot: u16) -> Result<Pine> {
        let p = socket_path(slot);
        let s = UnixStream::connect(&p).with_context(|| {
            format!("connecting to PINE socket {} (is PCSX2 running with Settings > Advanced > Enable PINE, slot {slot}?)", p.display())
        })?;
        Ok(Pine { s })
    }

    /// Sends one message (a concatenation of commands) and returns the reply payload after the status byte.
    fn call(&mut self, cmds: &[u8]) -> Result<Vec<u8>> {
        let mut msg = ((cmds.len() + 4) as u32).to_le_bytes().to_vec();
        msg.extend_from_slice(cmds);
        self.s.write_all(&msg)?;
        let mut hdr = [0u8; 5];
        self.s.read_exact(&mut hdr)?;
        let len = u32::from_le_bytes(hdr[..4].try_into().unwrap()) as usize;
        ensure!(len >= 5, "bad PINE reply length {len}");
        let mut body = vec![0u8; len - 5];
        self.s.read_exact(&mut body)?;
        if hdr[4] != 0 { bail!("PINE command failed (status {:#x}); is a game running?", hdr[4]); }
        Ok(body)
    }

    fn string(&mut self, op: u8) -> Result<String> {
        let r = self.call(&[op])?;
        ensure!(r.len() >= 4, "short PINE string reply");
        let s = &r[4..];
        Ok(String::from_utf8_lossy(&s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]).into_owned())
    }
    pub fn version(&mut self) -> Result<String> { self.string(MSG_VERSION) }
    pub fn game_id(&mut self) -> Result<String> { self.string(MSG_ID) }
    pub fn title(&mut self) -> Result<String> { self.string(MSG_TITLE) }
    /// 0 = running, 1 = paused, 2 = shutdown.
    pub fn status(&mut self) -> Result<u32> {
        let r = self.call(&[MSG_STATUS])?;
        Ok(u32::from_le_bytes(r.get(..4).context("short status")?.try_into().unwrap()))
    }
    pub fn status_name(&mut self) -> &'static str {
        match self.status() { Ok(0) => "running", Ok(1) => "paused", Ok(2) => "shutdown", _ => "unknown" }
    }
    /// Asks PCSX2 to save a state to `slot` (asynchronous on the emulator's CPU thread).
    pub fn save_state(&mut self, slot: u8) -> Result<()> { self.call(&[MSG_SAVE_STATE, slot]).map(|_| ()) }

    /// One message of `MsgRead64` at each of `addrs` (8-byte aligned, at most 50000): the 8 bytes at each, in
    /// order. PCSX2 serves a message's commands back to back, so a batch is as close to one snapshot as PINE
    /// gets (the EE keeps running meanwhile: callers bracket a batch with a counter read to detect a tick
    /// boundary inside it).
    pub fn read_many(&mut self, addrs: &[u32]) -> Result<Vec<[u8; 8]>> {
        ensure!(addrs.len() <= READS_PER_MSG, "too many reads in one PINE message");
        let mut cmds = Vec::with_capacity(addrs.len() * 5);
        for &a in addrs {
            ensure!(a.is_multiple_of(8), "PINE reads must be 8-byte aligned ({a:#x})");
            cmds.push(MSG_READ64);
            cmds.extend_from_slice(&a.to_le_bytes());
        }
        let r = self.call(&cmds)?;
        ensure!(r.len() == addrs.len() * 8, "PINE returned {} bytes for {} reads", r.len(), addrs.len());
        Ok(r.as_chunks::<8>().0.to_vec())
    }

    /// Writes `bytes` to EE memory at `addr` in one message of `MsgWrite8` (dev pokes: unlocking a planet).
    pub fn write(&mut self, addr: u32, bytes: &[u8]) -> Result<()> {
        ensure!(bytes.len() <= READS_PER_MSG, "too many bytes in one PINE message");
        let mut cmds = Vec::with_capacity(bytes.len() * 6);
        for (i, &b) in bytes.iter().enumerate() {
            cmds.push(MSG_WRITE8);
            cmds.extend_from_slice(&(addr + i as u32).to_le_bytes());
            cmds.push(b);
        }
        self.call(&cmds).map(|_| ())
    }

    /// Reads `len` bytes of EE memory at `addr` (both multiples of 8) through batched `MsgRead64`.
    pub fn read(&mut self, addr: u32, len: usize) -> Result<Vec<u8>> {
        ensure!(addr.is_multiple_of(8) && len.is_multiple_of(8), "PINE reads must be 8-byte aligned");
        let mut out = Vec::with_capacity(len);
        let mut a = addr;
        let end = addr as u64 + len as u64;
        while (a as u64) < end {
            let n = (((end - a as u64) / 8) as usize).min(READS_PER_MSG);
            let mut cmds = Vec::with_capacity(n * 5);
            for i in 0..n {
                cmds.push(MSG_READ64);
                cmds.extend_from_slice(&(a + 8 * i as u32).to_le_bytes());
            }
            let r = self.call(&cmds)?;
            ensure!(r.len() == n * 8, "PINE returned {} bytes for {} reads", r.len(), n);
            out.extend_from_slice(&r);
            a += 8 * n as u32;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    /// A fake PINE server answering MsgRead64 with the address itself, to check framing and batching.
    #[test]
    fn batched_read_framing() {
        let dir = std::env::temp_dir().join(format!("rc-trace-pine-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("pcsx2.sock");
        let _ = std::fs::remove_file(&path);
        let l = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut msgs = 0;
            loop {
                let mut h = [0u8; 4];
                if s.read_exact(&mut h).is_err() { break; }
                let len = u32::from_le_bytes(h) as usize;
                assert!(len < 650_000);
                let mut b = vec![0u8; len - 4];
                s.read_exact(&mut b).unwrap();
                let mut reply = vec![0u8; 5];
                for c in b.chunks(5) {
                    assert_eq!(c[0], MSG_READ64);
                    reply.extend_from_slice(&(u32::from_le_bytes(c[1..5].try_into().unwrap()) as u64).to_le_bytes());
                }
                assert!(reply.len() < 450_000);
                let n = reply.len() as u32;
                reply[..4].copy_from_slice(&n.to_le_bytes());
                s.write_all(&reply).unwrap();
                msgs += 1;
            }
            msgs
        });
        let mut c = Pine { s: UnixStream::connect(&path).unwrap() };
        let n = 120_000 * 8;
        let r = c.read(0x10_0000, n).unwrap();
        drop(c);
        assert_eq!(server.join().unwrap(), 3); // 50000 + 50000 + 20000
        for (i, w) in r.chunks(8).enumerate() {
            assert_eq!(u64::from_le_bytes(w.try_into().unwrap()), 0x10_0000 + 8 * i as u64);
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// `read_many`: one message, the replies in request order.
    #[test]
    fn read_many_framing() {
        let dir = std::env::temp_dir().join(format!("rc-trace-pine-many-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("pcsx2.sock");
        let _ = std::fs::remove_file(&path);
        let l = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut h = [0u8; 4];
            s.read_exact(&mut h).unwrap();
            let mut b = vec![0u8; u32::from_le_bytes(h) as usize - 4];
            s.read_exact(&mut b).unwrap();
            let mut reply = vec![0u8; 5];
            for c in b.chunks(5) { reply.extend_from_slice(&(!(u32::from_le_bytes(c[1..5].try_into().unwrap()) as u64)).to_le_bytes()); }
            let n = reply.len() as u32;
            reply[..4].copy_from_slice(&n.to_le_bytes());
            s.write_all(&reply).unwrap();
        });
        let mut c = Pine { s: UnixStream::connect(&path).unwrap() };
        let addrs = [0x15f5c8u32, 0x13f3d0, 0x100];
        let r = c.read_many(&addrs).unwrap();
        server.join().unwrap();
        for (a, w) in addrs.iter().zip(&r) { assert_eq!(u64::from_le_bytes(*w), !(*a as u64)); }
        assert!(c.read_many(&[3]).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
