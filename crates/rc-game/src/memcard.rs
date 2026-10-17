//! The memory card, natively: the card driver state machine `memcard_Update` (boot 0x2093d8 = L01 0x25f6a8), the card
//! monitor `fun_00208840` (boot 0x208840 = L01 0x25ec38, its 25 status handlers through the jump table L01 0x1849b8),
//! the slot previews (`memcard_RestoreInfo` 0x20ae60), `memcard_Save` (boot 0x20b178 = L01 0x261448) and the whole
//! saves of the menus (`fun_002269c0` Save page, `fun_00226a70` New Game, `fun_00226b08` challenge mode; L01 0x29a5c8 /
//! 0x29a678 / 0x29a710). Spec: docs/plan/game_state.md §3, docs/plan/progression.md `## saves`.
//!
//! **Native card.** The PS2 card is a folder ([`CardFs`]): `<save root>/BASCUS-97199RATCHET/` holding exactly the files
//! the game writes on a card (`icon.sys`, `static.ico`, `save0.bin`..`save4.bin`, the `BASCUS-97199RATCHET` reservation),
//! byte-compatible with a real card's. The libmc calls (`sceMcGetInfo`, `sceMcChdir`, `sceMcMkdir`, `sceMcOpen`,
//! `sceMcRead`, `sceMcWrite`, `sceMcSeek`, `sceMcClose`, `sceMcFormat`) are done synchronously on the folder and their
//! results handed to the machine one frame later through the same `sceMcSync` step the game polls (`0x13d35c`), so every
//! state and substate of `memcard_Update` runs in the game's order, one libmc call per frame. Only the latency differs
//! (a card takes many frames per call). A written file is replaced atomically on close (temp file + rename).
//!
//! **What reaches the game state.** The load (state 0xd/0xe) restores the global section and the 20 level sections into
//! [`GameState`] through `RestoreData` ([`GameState::restore_section`]); the error count lands in [`Card::errors`]. The
//! saves write the bytes `PrepData` / `MakeWholeSave` made **at the call** (the game prepares its buffers 0x14eed0 /
//! 0x1506d0 / the whole-save buffer when the save is asked for, and the card writes them frames later).
//!
//! ## Coverage (`memcard_Update` 0x2093d8; `r` = the last libmc result 0x13d34c)
//! | case | what | port |
//! |---|---|---|
//! | entry | busy 0x13d35c: `sceMcSync` → r, busy = not done; else busy = 1 | [`MemCard::update`] |
//! | 0 | `sceMcGetInfo(card)` (type, free, format) → 1 | ported (native info: type 2, free [`NATIVE_FREE`], formatted) |
//! | 1 | r ≠ 0: slot −3, info = r, errors −1; next card; card ≥ 1 → 2 | ported |
//! | 2 | idle: no request → card++ and re-poll after 0x1e; request → state = req (card = req card; r ≠ 0 on card 0 → error 0x271a) | ported |
//! | 3 / 4 | format (unformatted cards only) / result → error 1 | ported (`CardFs::format`: creates the root) |
//! | 5 / 6 | unformat | n/a: no caller requests 5 |
//! | 7 | `sceMcChdir(dir)` → 8 | ported |
//! | 8.0 | chdir r: 0 → slot −1 if < 0; ≠ 0 → slot −2, error 3 (−2) / 4 (other, −4 silent) | ported |
//! | 8.1–8.5 | open save0, read the 8-byte header, compare with `GetDataSize` (errors++ per size), close → 0x15; error codes 0x2711..0x2716, 3, 4 | ported |
//! | 9 / 10.0 | create: free < 0x15e → error 7 | ported |
//! | 10.1–10.2 | `sceMcMkdir` (0 or −4 ok; else 7 / 0xd / 6) | ported |
//! | 10.3–10.4 | read the `save_game` lump (stream) | ported (the lump is in memory: next frame) |
//! | 10.5–10.17 | icon.sys (0x3c4), static.ico, save0..4 = the template (`GetDataSize·…+8` bytes); write errors 0xb / 8 / 7 / 0xd / 6; close error 0xc | ported |
//! | 10.18–10.20 | `BASCUS-97199RATCHET`: 0x3c04 bytes (0x3c00 PAL) of RAM from 0x15ed84 | ported as a zero-filled file of that size [L: a reservation the game never reads; the RAM dump is not reproducible] |
//! | 10.21 | slot −1, errors 0 → state 0 | ported |
//! | 0xb / 0xc | delete the save folder | n/a: no caller requests 0xb |
//! | 0xd / 0xe | load: open save%d (slot), read header, global → `RestoreData`, 20 level sections → `RestoreData`; > 0x1800 / > 0x1000 traps; errors 0x14..0x1c | ported |
//! | 0xf / 0x10 | incremental save: open, seek 8, write global, seek level·size (cur), write that level; errors 0x1d, 0x1e..0x26, 0x2717 | ported |
//! | 0x11 / 0x12 | erase a slot to the template | n/a: no caller requests 0x11 |
//! | 0x13 / 0x14 | whole save of the prepared buffer; errors 0x2718, 0x27, 0x2b, 0xb, 0x28..0x2d | ported |
//! | 0x15..0x17 | the 5 slot previews: open, header, global section → `memcard_RestoreInfo` | ported |
//!
//! ## Coverage (the card monitor `fun_00208840`, status 0x15eeb0, flags 0x15eeb4)
//! Every status 0..24 is ported line by line in [`MemCard::monitor`] (handlers L01 0x25eca0..0x25f3e0, 0x25f320). The
//! flag bits: 1 card changed (notice shown), 2 save asked, 4 load asked, 8 format confirmed, 0x10 create confirmed, 0x20
//! "continue without saving" / cancel, 0x40 error dialog up, 0x80 / 0x100 / 0x200 save / load / auto-save failed.
//!
//! ## Coverage (`memcard_Save(force, pretend)` 0x261448)
//! | step | port |
//! |---|---|
//! | `sceCdReadClock` + the JST → local fix 0x12d6d8 into 0x15ee98 | [`SaveCapture::clock`] (the engine stamps the host's UTC time as BCD [L: no local-time fix natively]) |
//! | landmark capture `fun_00208770` | [`GameState::capture_landmarks`] |
//! | the map mask pack `fun_00207b08` (chunk 3002 of the current level) | the caller's `crate::map::MapState::pack` → [`SaveCapture::map_mask`] |
//! | no card / no slot (0x13d350 = −1 or slot < 0) → returns `force == 0` | ported |
//! | `checked |= force`; auto-save (force 0) raises flag 0x200 | ported |
//! | idle: pretend level / visited, preview, `PrepData` global + current level, put back, request 0xf | ported |
//! | returns `req == 0xf` | ported |

use crate::game_state::{GameState, Scope};
use rc_formats::save_game::{SaveGameLump, Section, LEVEL_SLOTS};
use std::path::{Path, PathBuf};

/// Save slots on a card (`save0.bin`..`save4.bin`).
pub const SLOTS: usize = 5;
/// The free clusters a new save needs (0x15e = 350 KB).
pub const MIN_FREE: i32 = 0x15e;
/// The free clusters the native card reports (8 MB card, empty).
pub const NATIVE_FREE: i32 = 8000;
/// The reservation file's size (NTSC 0x3c04, PAL 0x3c00).
pub const RESERVE_NTSC: usize = 0x3c04;
pub const RESERVE_PAL: usize = 0x3c00;
/// `sceMcGetInfo`'s card type of a PS2 memory card.
pub const TYPE_PS2: i32 = 2;
/// Trap limits of the load (`raise_kernel_trap` when a header size exceeds them).
const GLOBAL_LIMIT: usize = 0x1800;
const LEVEL_LIMIT: usize = 0x1000;

/// libmc result codes (`sceMcSync` results).
pub mod mc {
    pub const OK: i32 = 0;
    /// Card changed / not found variants.
    pub const NO_FORMAT: i32 = -2;
    pub const FULL: i32 = -3;
    pub const NOT_FOUND: i32 = -4;
    pub const DENIED: i32 = -5;
    pub const NO_CARD: i32 = -10;
}

/// The card's status messages (`0x15eeb0`), named after what the freeze dialog shows.
pub mod status {
    pub const INIT: i32 = 0;
    /// Checked and ready (`0x13d384` set: auto-save on).
    pub const READY: i32 = 1;
    pub const CHANGED: i32 = 2;
    pub const RECHECK: i32 = 3;
    pub const POLL: i32 = 4;
    pub const UNFORMATTED: i32 = 5;
    pub const FORMAT_ASK: i32 = 6;
    pub const FORMAT: i32 = 7;
    pub const FORMATTING: i32 = 8;
    pub const PRESENT: i32 = 9;
    pub const CHECK: i32 = 10;
    pub const CHECKING: i32 = 11;
    pub const NO_DATA: i32 = 12;
    pub const CREATE_ASK: i32 = 13;
    pub const CREATE: i32 = 14;
    pub const CREATING: i32 = 15;
    /// Data found (auto-save still off until a save or load).
    pub const FOUND: i32 = 16;
    pub const FORMAT_FAILED: i32 = 17;
    pub const CREATE_FAILED: i32 = 18;
    pub const NO_SPACE: i32 = 19;
    pub const LOAD_FAILED: i32 = 20;
    pub const SAVE_FAILED: i32 = 21;
    pub const AUTOSAVE: i32 = 22;
    pub const UNFORMATTED_FE: i32 = 23;
    pub const NO_DATA_FE: i32 = 24;
}

/// One slot's preview (`0x13d2b0 + card·0xb8 + slot·0x1c`, `memcard_RestoreInfo`): the global chunks 0..4 read from
/// fixed offsets of the slot's global section (+0x10 level, +0x1c bolts, +0x28 completes, +0x34 elapsed, +0x40 clock).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Preview {
    /// −1: empty slot (the template's level).
    pub level: i32,
    pub bolts: i32,
    pub completes: i32,
    /// 60 Hz ticks.
    pub elapsed: i32,
    /// `sceCdCLOCK` (BCD): [0] stat, [1] second, [2] minute, [3] hour, [4] pad, [5] day, [6] month, [7] year.
    pub clock: [u8; 8],
    /// +0x18: the global section's checksum failed.
    pub bad: bool,
}

impl Preview {
    /// `memcard_RestoreInfo` on a global section's bytes (header included).
    pub fn from_section(b: &[u8]) -> Preview {
        let w = |o: usize| b.get(o..o + 4).map_or(0, |x| i32::from_le_bytes(x.try_into().unwrap()));
        let ok = Section::decode(b).is_ok_and(|s| s.crc_ok);
        let mut clock = [0u8; 8];
        if let Some(c) = b.get(0x40..0x48) { clock.copy_from_slice(c); }
        Preview { level: w(0x10), bolts: w(0x1c), completes: w(0x28), elapsed: w(0x34), clock, bad: !ok }
    }
}

/// The card record (0x13d290, 0xb8 bytes per card; one card port, slot 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    /// +0x08 `sceMcGetInfo` type (2 = PS2 card), +0x0c free clusters, +0x10 formatted.
    pub ty: i32,
    pub free: i32,
    pub formatted: i32,
    /// +0x14 (0x13d2a4): the active save slot 0..4; −1 the folder exists with no slot chosen; −2 no folder / failure; −3
    /// no card.
    pub slot: i32,
    /// +0x18 (0x13d2a8): the preview loop's slot.
    pub preview_slot: i32,
    /// +0x1c (0x13d2ac): the last `sceMcGetInfo` result (0 same card, −1 new formatted card, −2 unformatted, ≤ −10 none).
    pub info: i32,
    /// +0x20 (0x13d2b0): the five previews.
    pub previews: [Preview; SLOTS],
    /// +0xac (0x13d33c): `RestoreData` errors of the last load (−1 no card).
    pub errors: i32,
    /// +0xb0 (0x13d340): the last header read (global, level section sizes).
    pub header: [u32; 2],
}

impl Default for Card {
    fn default() -> Self {
        // The boot ELF's initial data at 0x13d290: every word 0 except +0x14 (0x13d2a4) = −3, "no card checked yet". The
        // card monitor's status 11 (`mc_status_11`) waits while the slot is below −2, so a check never ends on this
        // initial value before `memcard_Update`'s chdir (state 8) has written −1 / −2 / a slot.
        Card { ty: 0, free: 0, formatted: 0, slot: -3, preview_slot: 0, info: 0, previews: [Preview::default(); SLOTS], errors: 0, header: [0; 2] }
    }
}

/// What a write sends (`0x13d37c`).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
enum WriteSrc {
    #[default]
    None,
    IconSys,
    StaticIco,
    Template,
    Reserve,
    /// The whole-save buffer (state 0x13).
    Whole,
}

/// A native libmc call made this frame; its result arrives at the next frame's sync.
type Pending = Option<i32>;

/// The folder standing for the memory card (module docs).
#[derive(Clone, Debug)]
pub struct CardFs {
    /// The folder that holds the card's directories (the save root); None = no card inserted.
    pub root: Option<PathBuf>,
    open: Vec<Option<OpenFile>>,
    /// `sceMcChdir`'s current directory (card path, e.g. `/BASCUS-97199RATCHET`).
    cwd: String,
}

#[derive(Clone, Debug)]
struct OpenFile {
    path: PathBuf,
    data: Vec<u8>,
    pos: usize,
    write: bool,
}

/// `sceMcOpen` modes.
pub mod open_mode {
    pub const READ: i32 = 1;
    pub const WRITE: i32 = 2;
    pub const CREATE_RDWR: i32 = 0x203;
}

impl CardFs {
    pub fn new(root: Option<PathBuf>) -> CardFs { CardFs { root, open: Vec::new(), cwd: "/".into() } }

    /// The host path of a card path (`/DIR/file`; relative paths resolve against the current directory).
    fn host(&self, p: &str) -> Option<PathBuf> {
        let root = self.root.as_ref()?;
        let full = if p.starts_with('/') { p.to_string() } else { format!("{}/{p}", self.cwd.trim_end_matches('/')) };
        let mut h = root.clone();
        for part in full.split('/').filter(|s| !s.is_empty()) {
            if part == ".." || part.contains('\\') { return None; }
            h.push(part);
        }
        Some(h)
    }

    /// `sceMcGetInfo`: (result, type, free, formatted). The root is created on first use (an empty, formatted card).
    pub fn get_info(&mut self) -> (i32, i32, i32, i32) {
        match &self.root {
            None => (mc::NO_CARD, 0, 0, 0),
            Some(r) => {
                let ok = std::fs::create_dir_all(r).is_ok();
                if ok { (mc::OK, TYPE_PS2, NATIVE_FREE, 1) } else { (mc::NO_CARD, 0, 0, 0) }
            }
        }
    }

    /// `sceMcFormat`: an empty card (the root created; nothing deleted).
    pub fn format(&mut self) -> i32 {
        match &self.root {
            Some(r) if std::fs::create_dir_all(r).is_ok() => mc::OK,
            _ => mc::NO_CARD,
        }
    }

    pub fn chdir(&mut self, dir: &str) -> i32 {
        let Some(h) = self.host(dir) else { return mc::NO_CARD };
        if h.is_dir() {
            self.cwd = dir.to_string();
            mc::OK
        } else {
            mc::NOT_FOUND
        }
    }

    pub fn mkdir(&mut self, dir: &str) -> i32 {
        let Some(h) = self.host(dir) else { return mc::NO_CARD };
        if h.is_dir() { return mc::NOT_FOUND; }
        match std::fs::create_dir_all(&h) {
            Ok(()) => mc::OK,
            Err(_) => mc::DENIED,
        }
    }

    /// `sceMcOpen(path, mode)`: a descriptor ≥ 0, or −4 (not found) / −5 (denied).
    pub fn open(&mut self, path: &str, mode: i32) -> i32 {
        let Some(h) = self.host(path) else { return mc::NO_CARD };
        let create = mode & 0x200 != 0;
        let write = mode & 2 != 0;
        let data = match std::fs::read(&h) {
            Ok(d) => d,
            Err(_) if create => {
                if h.parent().is_none_or(|p| !p.is_dir()) { return mc::NOT_FOUND; }
                Vec::new()
            }
            Err(_) => return mc::NOT_FOUND,
        };
        // 0x203 (create, read/write) truncates as the game's fresh files are written whole.
        let data = if create { Vec::new() } else { data };
        let f = OpenFile { path: h, data, pos: 0, write };
        let fd = match self.open.iter().position(Option::is_none) {
            Some(i) => {
                self.open[i] = Some(f);
                i
            }
            None => {
                self.open.push(Some(f));
                self.open.len() - 1
            }
        };
        fd as i32
    }

    pub fn read(&mut self, fd: i32, n: usize) -> (i32, Vec<u8>) {
        let Some(f) = usize::try_from(fd).ok().and_then(|i| self.open.get_mut(i)).and_then(Option::as_mut) else { return (mc::DENIED, Vec::new()) };
        let end = (f.pos + n).min(f.data.len());
        let out = f.data.get(f.pos..end).map(<[u8]>::to_vec).unwrap_or_default();
        f.pos = end.max(f.pos);
        (out.len() as i32, out)
    }

    pub fn write(&mut self, fd: i32, b: &[u8]) -> i32 {
        let Some(f) = usize::try_from(fd).ok().and_then(|i| self.open.get_mut(i)).and_then(Option::as_mut) else { return mc::DENIED };
        if !f.write { return mc::DENIED; }
        if f.data.len() < f.pos + b.len() { f.data.resize(f.pos + b.len(), 0); }
        f.data[f.pos..f.pos + b.len()].copy_from_slice(b);
        f.pos += b.len();
        b.len() as i32
    }

    /// `sceMcSeek(fd, offset, whence)`: 0 from the start, 1 from the current position. Returns the new position.
    pub fn seek(&mut self, fd: i32, off: i64, whence: i32) -> i32 {
        let Some(f) = usize::try_from(fd).ok().and_then(|i| self.open.get_mut(i)).and_then(Option::as_mut) else { return mc::DENIED };
        let base = if whence == 1 { f.pos as i64 } else { 0 };
        let p = (base + off).max(0) as usize;
        f.pos = p;
        p as i32
    }

    /// `sceMcClose`: a written file is replaced atomically.
    pub fn close(&mut self, fd: i32) -> i32 {
        let Some(f) = usize::try_from(fd).ok().and_then(|i| self.open.get_mut(i)).and_then(Option::take) else { return mc::DENIED };
        if !f.write { return mc::OK; }
        let tmp = f.path.with_extension("tmp-rerac");
        let ok = std::fs::write(&tmp, &f.data).and_then(|_| std::fs::rename(&tmp, &f.path)).is_ok();
        if ok { mc::OK } else { mc::DENIED }
    }

    /// The bytes of a card file (the slot previews of the tools; not a libmc call).
    pub fn read_file(&self, path: &str) -> Option<Vec<u8>> { std::fs::read(self.host(path)?).ok() }

    /// The boot's card check `fun_00209168` (before the logos): 0 ok, 1 no card / not a PS2 card, 2 no save folder and
    /// less than 350 KB free.
    pub fn boot_check(&mut self, dir: &str) -> i32 {
        let (r, ty, free, formatted) = self.get_info();
        if r == mc::DENIED || r < -9 { return 1; }
        if r != mc::NO_FORMAT {
            if ty != TYPE_PS2 { return 1; }
            if formatted != 0 {
                let has = self.host(dir).is_some_and(|h| h.is_dir());
                if has { return 0; }
                return ((free < MIN_FREE) as i32) << 1;
            }
        }
        0
    }

    fn file(dir: &str, name: &str) -> String { format!("{dir}/{name}") }
}

/// The values `memcard_Save` / the whole saves capture first (module docs).
#[derive(Clone, Debug, Default)]
pub struct SaveCapture {
    /// `sceCdReadClock` + the local-time fix (0x15ee98).
    pub clock: [u8; 8],
    /// The landmark hooks of the current level (0x179638: the level's hook slot k → the moby's x, y, angle) for
    /// `fun_00208770`, and the level ranges 0x1c4938 (landmark index = `base[level] + k`).
    pub hooks: Vec<(i32, (f32, f32, f32))>,
    pub base: Vec<i32>,
    /// `fun_00207b08`: the current level's packed map mask (chunk 3002), None when the level has no map state.
    pub map_mask: Option<[u8; crate::map::SAVE_BYTES]>,
}

/// The memory-card globals (0x13d348..0x13d384) and the monitor's (0x15eeb0, 0x15eeb4, 0x162280).
#[derive(Clone, Debug)]
pub struct MemCard {
    pub fs: CardFs,
    /// The card's directory (`memcard_GetName`, e.g. `/BASCUS-97199RATCHET`).
    pub dir: String,
    /// The disc's `save_game` lump (icon.sys, static.ico, the template).
    pub lump: Option<SaveGameLump>,
    /// PAL (`0x15ee90`): the reservation file's size.
    pub pal: bool,
    pub card: Card,
    /// 0x13d34c the last result, 0x13d350 the save card (0 or −1), 0x13d354 the card being worked on, 0x13d358 the level
    /// loop counter, 0x13d35c busy (a libmc call is pending), 0x13d360 the descriptor.
    pub result: i32,
    pub save_card: i32,
    pub cur: i32,
    pub level: i32,
    pub busy: bool,
    pub fd: i32,
    /// 0x13d364 state, 0x13d368 substate, 0x13d36c / 0x13d370 the requested state and card (−1 none).
    pub state: i32,
    pub sub: i32,
    pub req_state: i32,
    pub req_card: i32,
    /// 0x13d374 the error code (0 none; the freeze dialog's message), 0x13d378 its card.
    pub error: i32,
    pub error_card: i32,
    /// 0x13d384: checked (a save or load was made: auto-save on).
    pub checked: bool,
    /// 0x15eeb0 / 0x15eeb4 / 0x162280.
    pub status: i32,
    pub flags: u32,
    pub status_frames: i32,
    /// 0x15f5a8 (L01) / 0x15f5e8 (boot): in the front end (`startlevel` sets it).
    pub front_end: bool,
    write_src: WriteSrc,
    len: usize,
    pending: Pending,
    /// The bytes the last read returned (the header / section buffers 0x13d340, 0x14eed0, 0x1506d0).
    read_buf: Vec<u8>,
    /// `PrepData` of the global section and of one level section (0x14eed0 / 0x1506d0), made by `memcard_Save`.
    pub buf_global: Vec<u8>,
    pub buf_level: Vec<u8>,
    /// The whole-save buffer state 0x13 writes (`MakeWholeSave` into the widget's stream buffer / 0x1ba250).
    pub whole: Vec<u8>,
    /// Status 22 found the auto-save failed: `mode_freezeInit(3, 0)` (taken by [`MemCard::take_freeze_request`]).
    freeze_request: bool,
}

impl MemCard {
    /// A card in `fs`, directory `dir` (`memcard_Init` + `memcard_GetName`); the state machine at rest (state 0).
    pub fn new(fs: CardFs, dir: String, lump: Option<SaveGameLump>) -> MemCard {
        MemCard {
            fs,
            dir,
            lump,
            pal: false,
            card: Card::default(),
            result: 0,
            save_card: 0,
            cur: 0,
            level: 0,
            busy: false,
            fd: -1,
            state: 0,
            sub: 0,
            req_state: -1,
            req_card: -1,
            error: 0,
            error_card: 0,
            checked: false,
            status: 0,
            flags: 0,
            status_frames: 0,
            front_end: false,
            write_src: WriteSrc::None,
            len: 0,
            pending: None,
            read_buf: Vec::new(),
            buf_global: Vec::new(),
            buf_level: Vec::new(),
            whole: Vec::new(),
            freeze_request: false,
        }
    }

    /// No card at all (deterministic runs, tests).
    pub fn absent() -> MemCard { MemCard::new(CardFs::new(None), "/BASCUS-97199RATCHET".into(), None) }

    /// The machine is idle and nothing is queued (`state < 3 && req < 0`), the menus' gate.
    pub fn idle(&self) -> bool { self.state < 3 && self.req_state < 0 }

    /// Requests `state` on card 0 if nothing is queued (`if (0x13d36c < 0) { 0x13d370 = 0; 0x13d36c = state; }`).
    pub fn request(&mut self, state: i32) {
        if self.req_state < 0 {
            self.req_card = 0;
            self.req_state = state;
        }
    }

    fn call(&mut self, r: i32) { self.pending = Some(r); }

    fn path(&self, name: &str) -> String { CardFs::file(&self.dir, name) }

    fn slot_file(&self, slot: i32) -> String { self.path(&rc_formats::save_game::save_file_name(slot.max(0) as usize)) }

    fn global_size(gs: &GameState) -> usize { gs.tables.global_size() }
    fn level_size(gs: &GameState) -> usize { gs.tables.level_size() }

    /// Ends the machine: `0x13d35c = 0; state = 0; card = 0` (LAB_0020ab58).
    fn rest(&mut self) {
        self.busy = false;
        self.state = 0;
        self.cur = 0;
    }

    /// Error `code` on the current card, machine to rest (`LAB_0020a690` after the error card is set).
    fn fail(&mut self, code: i32) {
        self.error_card = self.cur;
        self.error = code;
        self.rest();
    }

    /// `LAB_0020a9c4`: error `code` (the error card left as it was), machine to rest.
    fn fail_nc(&mut self, code: i32) {
        self.error = code;
        self.rest();
    }

    /// `LAB_0020a878`: a short read / write — close, then rest (the error code already set).
    fn fail_close(&mut self) {
        self.cur = self.error_card;
        let r = self.fs.close(self.fd);
        self.call(r);
        self.rest();
    }

    /// One frame of `memcard_Update` (0x2093d8). `gs` receives the loads.
    pub fn update(&mut self, gs: &mut GameState) {
        if self.busy {
            // sceMcSync: the native call completed when it was made.
            self.result = self.pending.take().unwrap_or(0);
            self.busy = false;
            return;
        }
        self.busy = true;
        match self.state {
            0 => {
                if self.cur < 0 { self.cur = 0; }
                let (r, ty, free, formatted) = self.fs.get_info();
                self.card.ty = ty;
                self.card.free = free;
                self.card.formatted = formatted;
                self.call(r);
                self.state = 1;
            }
            1 => {
                if self.result != 0 {
                    self.card.slot = -3;
                    self.card.info = self.result;
                    self.card.errors = -1;
                }
                self.cur += 1;
                self.state = if self.cur < 1 { 0 } else { 2 };
                self.busy = false;
            }
            2 => {
                if self.req_state < 0 {
                    self.cur += 1;
                    if self.cur > 0x1e {
                        self.state = 0;
                        self.cur = 0;
                    }
                } else {
                    if self.card.info == 0 {
                        if self.req_card < 0 {
                            self.error = 0x271a;
                        } else {
                            self.cur = self.req_card;
                            self.state = self.req_state;
                            self.error = 0;
                        }
                    }
                    self.req_card = -1;
                    self.req_state = -1;
                }
                self.busy = false;
            }
            3 => {
                if self.card.formatted == 0 {
                    let r = self.fs.format();
                    self.call(r);
                    self.state = 4;
                    return;
                }
                self.busy = false;
                self.state = 2;
            }
            4 => {
                if self.result != 0 {
                    self.error_card = self.cur;
                    self.error = 1;
                }
                self.rest();
            }
            7 => {
                let dir = self.dir.clone();
                let r = self.fs.chdir(&dir);
                self.call(r);
                self.sub = 0;
                self.state = 8;
            }
            8 => self.check_step(gs),
            9 => {
                self.state = 10;
                self.sub = 0;
                self.card.slot = 0;
                self.create_step(gs);
            }
            10 => self.create_step(gs),
            0xd => {
                if self.card.slot < 0 {
                    self.fail_nc(0x13);
                    return;
                }
                self.level = 0;
                self.state = 0xe;
                self.sub = 0;
                self.read_step(gs);
            }
            0xe | 0x17 => self.read_step(gs),
            0xf => {
                if self.card.errors != 0 {
                    self.fail_nc(0x2717);
                } else if self.card.slot < 0 {
                    self.fail_nc(0x1d);
                } else {
                    self.sub = 0;
                    self.state = 0x10;
                    self.save_step(gs);
                }
            }
            0x10 => self.save_step(gs),
            0x13 => {
                if self.card.errors != 0 {
                    self.fail_nc(0x2718);
                } else if self.card.slot < 0 {
                    self.fail_nc(0x27);
                } else {
                    self.level = 0;
                    self.state = 0x14;
                    self.sub = 0;
                    self.whole_step(gs);
                }
            }
            0x14 => self.whole_step(gs),
            0x15 => {
                self.state = 0x16;
                self.card.preview_slot = -1;
                self.level = 0;
            }
            0x16 => {
                self.card.preview_slot += 1;
                if self.card.preview_slot < SLOTS as i32 {
                    self.sub = 0;
                    self.state = 0x17;
                } else {
                    self.rest();
                }
            }
            // 5 / 6 (unformat), 0xb / 0xc (delete), 0x11 / 0x12 (erase): no caller requests them.
            _ => {}
        }
    }

    /// State 8 (after `sceMcChdir`): the header of save0 against the chunk tables.
    fn check_step(&mut self, gs: &GameState) {
        match self.sub {
            0 => {
                if self.result == 0 {
                    if self.card.slot < 0 { self.card.slot = -1; }
                    self.busy = false;
                    self.sub = 1;
                    return;
                }
                self.card.slot = -2;
                let code = if self.result == mc::NO_FORMAT { 3 } else { 4 };
                if !(code == 4 && self.result == mc::NOT_FOUND) {
                    self.error_card = self.cur;
                    self.error = code;
                }
                self.rest();
            }
            1 => {
                let p = self.slot_file(0);
                let r = self.fs.open(&p, open_mode::READ);
                self.call(r);
                self.sub = 2;
            }
            2 => {
                if self.result >= 0 {
                    self.sub = 3;
                    self.fd = self.result;
                    self.busy = false;
                    return;
                }
                self.card.slot = -2;
                self.error_card = self.cur;
                self.error = match self.result {
                    -7 => 10000,
                    mc::DENIED => 0x2711,
                    mc::NOT_FOUND => 0x2712,
                    mc::FULL => 0x2713,
                    mc::NO_FORMAT => 3,
                    _ => 4,
                };
                self.cur = 0;
                self.state = 0;
                self.busy = false;
            }
            3 => {
                self.len = 8;
                let (r, b) = self.fs.read(self.fd, 8);
                self.read_buf = b;
                self.call(r);
                self.sub = 4;
            }
            4 => {
                if self.result == self.len as i32 {
                    self.card.errors = 0;
                    self.card.header = header(&self.read_buf);
                    if self.card.header[0] as usize != Self::global_size(gs) { self.card.errors += 1; }
                    if self.card.header[1] as usize != Self::level_size(gs) { self.card.errors += 1; }
                    self.busy = false;
                    self.sub = 5;
                    return;
                }
                self.card.slot = -2;
                self.error_card = self.cur;
                if self.result >= 0 {
                    self.error = 0x2716;
                    self.fail_close();
                    return;
                }
                self.error = match self.result {
                    mc::DENIED => 0x2711,
                    mc::NOT_FOUND => 0x2714,
                    mc::FULL => 0x2715,
                    mc::NO_FORMAT => 3,
                    _ => 4,
                };
                self.rest();
            }
            5 => {
                let r = self.fs.close(self.fd);
                self.call(r);
                self.busy = false;
                self.state = 0x15;
            }
            _ => {}
        }
    }

    /// States 9 / 10: create the folder and its files.
    fn create_step(&mut self, gs: &GameState) {
        match self.sub {
            0 => {
                if self.card.free < MIN_FREE {
                    self.error = 7;
                    self.error_card = self.cur;
                    self.cur = 0;
                    self.state = 0;
                } else {
                    self.sub = 1;
                }
                self.busy = false;
            }
            1 => {
                let dir = self.dir.clone();
                let r = self.fs.mkdir(&dir);
                self.call(r);
                self.sub = 2;
            }
            2 => {
                if self.result == 0 || self.result == mc::NOT_FOUND {
                    self.sub = 3;
                    return;
                }
                self.error_card = self.cur;
                self.error = match self.result {
                    mc::FULL => 7,
                    mc::NO_FORMAT => 6,
                    _ => 0xd,
                };
                self.rest();
            }
            // The save_game lump's read (the stream; in memory here).
            3 => {
                self.busy = false;
                self.sub = 4;
            }
            4 => {
                self.sub = 5;
                self.busy = false;
            }
            5 | 9 | 0xd | 0x12 => {
                let name = match self.sub {
                    5 => rc_formats::save_game::ICON_SYS_NAME.to_string(),
                    9 => rc_formats::save_game::STATIC_ICO_NAME.to_string(),
                    0xd => rc_formats::save_game::save_file_name(self.card.slot.max(0) as usize),
                    _ => self.dir.trim_start_matches('/').to_string(),
                };
                let p = self.path(&name);
                let r = self.fs.open(&p, open_mode::CREATE_RDWR);
                self.call(r);
                self.fd = -1;
                self.sub += 1;
            }
            6 | 10 | 0xe | 0x13 => {
                if self.result < 0 {
                    self.fail(10);
                    return;
                }
                if self.fd < 0 { self.fd = self.result; }
                let (src, len) = match self.sub {
                    10 => (WriteSrc::StaticIco, self.lump.as_ref().map_or(0, |l| l.static_ico.len())),
                    6 => (WriteSrc::IconSys, 0x3c4),
                    0xe => (WriteSrc::Template, Self::global_size(gs) + Self::level_size(gs) * LEVEL_SLOTS + 8),
                    _ => (WriteSrc::Reserve, if self.pal { RESERVE_PAL } else { RESERVE_NTSC }),
                };
                self.write_src = src;
                self.len = len;
                let bytes = self.source_bytes(len);
                let r = self.fs.write(self.fd, &bytes);
                self.call(r);
                self.sub += 1;
            }
            7 | 0xb | 0xf | 0x14 => {
                if self.result == self.len as i32 {
                    let r = self.fs.close(self.fd);
                    self.call(r);
                    self.sub += 1;
                    return;
                }
                self.error_card = self.cur;
                self.error = 0xb;
                if self.result >= 0 {
                    self.fail_close();
                    return;
                }
                self.error = match self.result {
                    mc::NOT_FOUND => 8,
                    mc::FULL => 7,
                    mc::NO_FORMAT => 6,
                    _ => 0xd,
                };
                self.rest();
            }
            8 | 0xc | 0x10 => {
                if self.result != 0 {
                    self.error = 0xc;
                    self.error_card = self.cur;
                    self.cur = 0;
                    self.state = 0;
                } else {
                    self.sub += 1;
                }
                self.busy = false;
            }
            0x11 => {
                self.card.slot += 1;
                self.sub = if self.card.slot < SLOTS as i32 { 0xd } else { 0x12 };
                self.busy = false;
            }
            // LAB_0020ab60: state 0 with the last close still to sync.
            0x15 => {
                self.card.slot = -1;
                self.card.errors = 0;
                self.state = 0;
                self.cur = 0;
            }
            _ => {}
        }
    }

    /// The bytes a create / whole-save write sends.
    fn source_bytes(&self, len: usize) -> Vec<u8> {
        let mut v = match self.write_src {
            WriteSrc::IconSys => self.lump.as_ref().map(|l| l.icon_sys.clone()).unwrap_or_default(),
            WriteSrc::StaticIco => self.lump.as_ref().map(|l| l.static_ico.clone()).unwrap_or_default(),
            WriteSrc::Template => self.lump.as_ref().map(|l| l.template.clone()).unwrap_or_default(),
            WriteSrc::Whole => self.whole.clone(),
            WriteSrc::Reserve | WriteSrc::None => Vec::new(),
        };
        v.resize(len, 0);
        v
    }

    /// States 0xe (load) and 0x17 (one slot's preview).
    fn read_step(&mut self, gs: &mut GameState) {
        let preview = self.state == 0x17;
        let read_err = |r: i32| match r {
            mc::DENIED => 0x15,
            mc::NOT_FOUND => 0x19,
            mc::FULL => 0x1a,
            mc::NO_FORMAT => 0x18,
            _ => 0x1c,
        };
        match self.sub {
            0 => {
                let slot = if preview { self.card.preview_slot } else { self.card.slot };
                let p = self.slot_file(slot);
                let r = self.fs.open(&p, open_mode::READ);
                self.call(r);
                self.sub = 1;
            }
            1 => {
                if self.result >= 0 {
                    self.fd = self.result;
                    self.sub = 2;
                    self.busy = false;
                    return;
                }
                let code = match self.result {
                    -7 => 0x14,
                    mc::DENIED => 0x15,
                    mc::NOT_FOUND => 0x16,
                    mc::FULL => 0x17,
                    mc::NO_FORMAT => 0x18,
                    _ => 0x1c,
                };
                self.error_card = self.cur;
                self.error = code;
                self.cur = 0;
                self.state = 0;
                self.busy = false;
            }
            2 => {
                self.len = 8;
                let (r, b) = self.fs.read(self.fd, 8);
                self.read_buf = b;
                self.call(r);
                self.sub = 3;
            }
            3 | 5 | 7 => {
                if self.result == self.len as i32 {
                    match self.sub {
                        3 => {
                            self.card.header = header(&self.read_buf);
                            self.busy = false;
                            self.sub = 4;
                            return;
                        }
                        5 => {
                            if preview {
                                let slot = self.card.preview_slot.clamp(0, SLOTS as i32 - 1) as usize;
                                self.card.previews[slot] = Preview::from_section(&self.read_buf);
                                self.sub = 8;
                            } else {
                                let e = restore(gs, Scope::Global, &self.read_buf);
                                self.card.errors = e as i32;
                                self.sub = 6;
                            }
                        }
                        _ => {
                            let e = restore(gs, Scope::Level(self.level.clamp(0, LEVEL_SLOTS as i32 - 1) as usize), &self.read_buf);
                            self.card.errors += e as i32;
                            self.level += 1;
                            self.sub = if self.level < LEVEL_SLOTS as i32 { 6 } else { 8 };
                        }
                    }
                    self.busy = false;
                    return;
                }
                self.error_card = self.cur;
                self.error = 0x1b;
                if self.result >= 0 {
                    self.fail_close();
                    return;
                }
                self.error = read_err(self.result);
                self.rest();
            }
            4 => {
                self.len = self.card.header[0] as usize;
                // raise_kernel_trap: a corrupt header stops the game; natively the load fails (error 0x1b).
                if self.len > GLOBAL_LIMIT {
                    self.error_card = self.cur;
                    self.error = 0x1b;
                    self.fail_close();
                    return;
                }
                let (r, b) = self.fs.read(self.fd, self.len);
                self.read_buf = b;
                self.call(r);
                self.sub = 5;
            }
            6 => {
                self.len = self.card.header[1] as usize;
                if self.len > LEVEL_LIMIT {
                    self.error_card = self.cur;
                    self.error = 0x1b;
                    self.fail_close();
                    return;
                }
                let (r, b) = self.fs.read(self.fd, self.len);
                self.read_buf = b;
                self.call(r);
                self.sub = 7;
            }
            8 => {
                let r = self.fs.close(self.fd);
                self.call(r);
                if preview {
                    self.busy = false;
                    self.state = 0x16;
                } else {
                    self.cur = 0;
                    self.state = 0;
                }
            }
            _ => {}
        }
    }

    /// State 0x10: the incremental save of the prepared buffers.
    fn save_step(&mut self, gs: &GameState) {
        let write_err = |r: i32| match r {
            mc::DENIED => 0x1f,
            mc::NOT_FOUND => 0x23,
            mc::FULL => 0x24,
            mc::NO_FORMAT => 0x22,
            _ => 0x26,
        };
        match self.sub {
            0 => {
                let p = self.slot_file(self.card.slot);
                let r = self.fs.open(&p, open_mode::WRITE);
                if r >= 0 { self.fd = r; }
                self.call(r);
                self.sub = 1;
            }
            1 => {
                if self.result < 0 {
                    self.error_card = self.cur;
                    self.error = match self.result {
                        -7 => 0x1e,
                        mc::DENIED => 0x1f,
                        mc::NOT_FOUND => 0x20,
                        mc::FULL => 0x21,
                        mc::NO_FORMAT => 0x22,
                        _ => 0x26,
                    };
                    self.cur = 0;
                    self.state = 0;
                    self.busy = false;
                    return;
                }
                self.fd = self.result;
                self.sub = 2;
                self.busy = false;
            }
            2 => {
                let r = self.fs.seek(self.fd, 8, 0);
                self.call(r);
                self.sub = 3;
            }
            3 | 7 => {
                if self.result >= 0 {
                    self.sub += 1;
                    return;
                }
                self.rest();
            }
            4 => {
                let b = self.buf_global.clone();
                let r = self.fs.write(self.fd, &b);
                self.call(r);
                self.len = Self::global_size(gs);
                self.sub = 5;
            }
            5 | 9 => {
                let want = if self.sub == 5 { Self::global_size(gs) } else { Self::level_size(gs) };
                if self.result == want as i32 {
                    self.sub += 1;
                    return;
                }
                self.error_card = self.cur;
                if self.result >= 0 {
                    self.error = 0x25;
                    self.fail_close();
                    return;
                }
                self.error = if self.sub == 9 && !matches!(self.result, mc::DENIED | mc::NOT_FOUND | mc::FULL) {
                    if self.result == mc::NO_FORMAT { 0x18 } else { 0x1c }
                } else {
                    write_err(self.result)
                };
                self.rest();
            }
            6 => {
                if self.level == 0 {
                    self.sub = 8;
                    return;
                }
                let off = self.level as i64 * Self::level_size(gs) as i64;
                let r = self.fs.seek(self.fd, off, 1);
                self.call(r);
                self.sub = 7;
            }
            8 => {
                let b = self.buf_level.clone();
                let r = self.fs.write(self.fd, &b);
                self.call(r);
                self.sub = 9;
            }
            10 => {
                let r = self.fs.close(self.fd);
                self.call(r);
                self.cur = 0;
                self.state = 0;
            }
            _ => {}
        }
    }

    /// State 0x14: the whole save of the prepared buffer.
    fn whole_step(&mut self, gs: &GameState) {
        match self.sub {
            0 => {
                let p = self.slot_file(self.card.slot);
                let r = self.fs.open(&p, open_mode::WRITE);
                self.call(r);
                self.sub = 1;
            }
            1 => {
                if self.result < 0 {
                    let c = self.cur;
                    self.cur = 0;
                    self.busy = false;
                    self.state = 0;
                    self.error = 0x2b;
                    self.error_card = c;
                    return;
                }
                self.fd = self.result;
                self.len = Self::global_size(gs) + Self::level_size(gs) * LEVEL_SLOTS + 8;
                self.write_src = WriteSrc::Whole;
                let bytes = self.source_bytes(self.len);
                let r = self.fs.write(self.fd, &bytes);
                self.call(r);
                self.sub = 2;
            }
            2 => {
                if self.result == self.len as i32 {
                    let r = self.fs.close(self.fd);
                    self.call(r);
                    self.sub = 3;
                    return;
                }
                self.error_card = self.cur;
                self.error = 0xb;
                if self.result >= 0 {
                    self.fail_close();
                    return;
                }
                self.error = match self.result {
                    mc::NOT_FOUND => 0x28,
                    mc::FULL => 0x29,
                    mc::NO_FORMAT => 0x2a,
                    _ => 0x2d,
                };
                self.rest();
            }
            3 => {
                let c = self.cur;
                self.cur = 0;
                self.busy = false;
                self.state = 0;
                if self.result != 0 {
                    self.error = 0x2c;
                    self.error_card = c;
                }
            }
            _ => self.busy = true,
        }
    }

    // -------------------------------------------------------------------------------------------------
    // The card monitor.

    /// One frame of `fun_00208840`: the handler of the current status, then the status' frame count (0x162280).
    pub fn monitor(&mut self) {
        let before = self.status;
        self.monitor_step();
        self.status_frames += 1;
        if self.status != before { self.status_frames = 0; }
    }

    fn monitor_step(&mut self) {
        use status::*;
        let info = self.card.info;
        match self.status {
            INIT => {
                self.status = RECHECK;
                self.checked = false;
                self.card.info = self.result;
            }
            READY => {
                let f = self.flags & !6;
                self.flags = f;
                if !self.checked {
                    self.status = RECHECK;
                    return;
                }
                if self.flags & 0x80 != 0 {
                    self.status = SAVE_FAILED;
                    self.flags = (f ^ 0x80) | 0x40;
                } else if self.flags & 0x100 != 0 {
                    self.status = LOAD_FAILED;
                    self.flags = (f ^ 0x100) | 0x40;
                } else if info != 0 {
                    self.checked = false;
                    self.status = CHANGED;
                    self.flags = f | 1;
                } else if self.flags & 0x200 != 0 {
                    self.status = AUTOSAVE;
                    self.flags = f ^ 0x200;
                }
            }
            CHANGED => {
                if self.flags & 1 == 0 { self.status = RECHECK; }
            }
            RECHECK => {
                self.req_card = -1;
                self.status = POLL;
                self.req_state = -1;
            }
            POLL => {
                self.flags &= !0x20;
                match info {
                    0 => self.status = PRESENT,
                    -1 => {
                        self.card.info = 0;
                        self.status = PRESENT;
                    }
                    mc::NO_FORMAT => self.status = UNFORMATTED,
                    _ => {}
                }
            }
            UNFORMATTED => {
                if info != mc::NO_FORMAT {
                    self.status = RECHECK;
                } else if self.flags & 2 != 0 {
                    self.status = FORMAT_ASK;
                }
            }
            FORMAT_ASK => {
                if info != mc::NO_FORMAT {
                    self.status = RECHECK;
                } else if self.flags & 0x20 != 0 {
                    self.flags ^= 0x20;
                    self.status = if self.front_end { UNFORMATTED_FE } else { UNFORMATTED };
                } else if self.flags & 8 != 0 {
                    self.status = FORMAT;
                    self.flags ^= 8;
                }
            }
            FORMAT => {
                self.card.info = 0;
                if self.req_state < 0 {
                    self.req_card = 0;
                    self.req_state = 3;
                }
                self.status = FORMATTING;
            }
            FORMATTING => {
                if self.state == 2 && self.req_state < 0 {
                    if self.error != 0 {
                        self.status = FORMAT_FAILED;
                        self.flags |= 0x40;
                    } else {
                        self.status = CREATE;
                    }
                }
            }
            PRESENT => {
                if info != 0 {
                    self.status = RECHECK;
                } else if self.flags & 6 != 0 {
                    self.status = CHECK;
                }
            }
            CHECK => {
                if self.state == 2 && self.req_state < 0 {
                    self.req_state = 7;
                    self.req_card = 0;
                    self.status = CHECKING;
                }
            }
            CHECKING => {
                if info < -1 {
                    self.status = RECHECK;
                } else if self.card.slot == -2 {
                    self.status = if self.card.free < MIN_FREE { NO_SPACE } else { NO_DATA };
                } else if self.card.slot > -2 {
                    self.status = FOUND;
                }
            }
            NO_DATA => {
                if info != 0 {
                    self.status = RECHECK;
                } else if self.flags & 2 != 0 {
                    self.status = CREATE_ASK;
                }
            }
            CREATE_ASK => {
                if info != 0 {
                    self.status = RECHECK;
                } else if self.flags & 0x20 != 0 {
                    self.flags ^= 0x20;
                    self.status = if self.front_end { NO_DATA_FE } else { NO_DATA };
                } else if self.flags & 0x10 != 0 {
                    self.status = CREATE;
                    self.flags ^= 0x10;
                }
            }
            CREATE => {
                if self.state == 2 && self.req_state < 0 {
                    self.req_state = 9;
                    self.req_card = 0;
                    self.status = CREATING;
                }
            }
            CREATING => {
                if self.state == 2 && self.req_state < 0 {
                    if self.error != 0 {
                        self.status = CREATE_FAILED;
                        self.flags |= 0x40;
                        return;
                    }
                    self.req_state = 7;
                    self.req_card = 0;
                    self.status = FOUND;
                    self.save_card = 0;
                    self.card.slot = 0;
                }
            }
            FOUND => {
                let f = self.flags & !6;
                self.flags = f;
                if self.flags & 0x80 != 0 {
                    self.status = SAVE_FAILED;
                    self.flags = (f ^ 0x80) | 0x40;
                } else if self.flags & 0x100 != 0 {
                    self.status = LOAD_FAILED;
                    self.flags = (f ^ 0x100) | 0x40;
                } else if info == 0 {
                    if self.checked { self.status = READY; }
                } else {
                    self.status = RECHECK;
                }
            }
            FORMAT_FAILED | CREATE_FAILED | LOAD_FAILED | SAVE_FAILED => {
                if self.flags & 0x40 == 0 { self.status = RECHECK; }
            }
            NO_SPACE => {
                if info != 0 { self.status = RECHECK; }
            }
            AUTOSAVE => {
                if self.state < 3 && self.req_state < 0 {
                    if self.error == 0 {
                        self.status = READY;
                    } else {
                        // mode_freezeInit(3, 0): the caller sees `freeze_request`.
                        self.freeze_request = true;
                        self.status = SAVE_FAILED;
                        self.flags |= 0x40;
                    }
                }
            }
            UNFORMATTED_FE => {
                if info != mc::NO_FORMAT {
                    self.status = RECHECK;
                } else if self.flags & 0x20 != 0 {
                    self.status = UNFORMATTED;
                    self.flags ^= 0x20;
                }
            }
            NO_DATA_FE => {
                if info != 0 {
                    self.status = RECHECK;
                } else if self.flags & 0x20 != 0 {
                    self.status = NO_DATA;
                    self.flags ^= 0x20;
                }
            }
            _ => {}
        }
    }

    /// The status allows the Save / Load pages without the dialog (`0x15eeb0 ∈ {1, 0x10}`).
    pub fn ready(&self) -> bool { matches!(self.status, status::READY | status::FOUND) }

    // -------------------------------------------------------------------------------------------------
    // The saves.

    /// The preview of the active slot from the live state (`*(slot·0x1c + 0x13d2b0..)` = level, bolts, elapsed, clock,
    /// completes), as every save path writes it.
    fn preview_from(&mut self, gs: &GameState, level: i32) {
        if let Some(p) = usize::try_from(self.card.slot).ok().and_then(|s| self.card.previews.get_mut(s)) {
            p.bolts = gs.global.bolts;
            p.level = level;
            p.elapsed = gs.global.elapsed;
            p.clock = gs.global.save_clock;
            p.completes = gs.global.completes;
        }
    }

    /// `memcard_Save(force, pretend)` 0x261448 (the capture of `cap` first). Returns the game's value: `force == 0`
    /// without a card / slot, else whether the incremental save was queued (state 0xf requested).
    pub fn memcard_save(&mut self, gs: &mut GameState, force: bool, pretend: i32, cap: &SaveCapture) -> bool {
        capture(gs, cap);
        let slot_ok = self.save_card != -1 && self.card.slot >= 0;
        if !slot_ok { return !force; }
        self.checked |= force;
        if self.checked {
            if !force { self.flags |= 0x200; }
            if self.idle() {
                self.level = gs.global.level;
                let cur = gs.global.level;
                let mut visited_was = None;
                if pretend >= 0 {
                    gs.global.level = pretend;
                    if let Some(l) = gs.levels.get_mut(pretend as usize) {
                        visited_was = Some(l.visited);
                        if l.visited == 0 { l.visited = 1; }
                    }
                }
                self.preview_from(gs, gs.global.level);
                self.buf_global = gs.global_section().encode();
                self.buf_level = usize::try_from(self.level).ok().filter(|&l| l < LEVEL_SLOTS).map(|l| gs.level_section(l).encode()).unwrap_or_default();
                if pretend >= 0 {
                    if let (Some(v), Some(l)) = (visited_was, gs.levels.get_mut(pretend as usize)) { l.visited = v; }
                    gs.global.level = cur;
                }
                if self.req_state < 0 {
                    self.req_state = 0xf;
                    self.req_card = self.save_card;
                }
            }
        }
        self.req_state == 0xf
    }

    /// The Save page's whole save `fun_002269c0(buffer, slot)`: capture, `MakeWholeSave`, checked, slot, request 0x13.
    pub fn save_whole(&mut self, gs: &mut GameState, slot: i32, cap: &SaveCapture) {
        capture(gs, cap);
        self.whole = gs.to_save_file().to_bytes();
        self.checked = true;
        self.card.slot = slot;
        self.save_card = 0;
        self.request(0x13);
    }

    /// New Game into a slot `fun_00226a70(buffer, slot)`: the new-game reset, the clock, `MakeWholeSave`, the slot's preview
    /// level 0, checked, request 0x13. `reset` is `load_and_initialize_level_chunk` (the engine's template restore).
    pub fn new_game_save(&mut self, gs: &mut GameState, slot: i32, clock: [u8; 8], reset: impl FnOnce(&mut GameState)) {
        reset(gs);
        gs.global.save_clock = clock;
        self.whole = gs.to_save_file().to_bytes();
        self.save_card = 0;
        self.card.slot = slot;
        if let Some(p) = usize::try_from(slot).ok().and_then(|s| self.card.previews.get_mut(s)) { p.level = 0; }
        self.checked = true;
        self.request(0x13);
    }

    /// Challenge mode `fun_00226b08(slot)`: [`GameState::challenge_reset`] (keeping the owned flags of `keep_owned`, the
    /// list L01 0x1ba120), the clock, then with a slot ≥ 0 its preview level 0 and the whole save of the new state (made
    /// into the ending buffer 0x1ba250, which the card writes).
    pub fn challenge_save(&mut self, gs: &mut GameState, slot: i32, clock: [u8; 8], keep_owned: &[usize], reset: impl FnOnce(&mut GameState)) {
        gs.challenge_reset(keep_owned, reset);
        gs.global.save_clock = clock;
        if slot >= 0 {
            self.card.slot = slot;
            if let Some(p) = self.card.previews.get_mut(slot as usize) { p.level = 0; }
            // MakeWholeSave(*0x1ba250): the new state into the ending buffer, which the card then writes.
            self.whole = gs.to_save_file().to_bytes();
            self.save_card = 0;
            self.request(0x13);
        }
    }

    /// The Load page's start (`LoadingDataMenu` ✕): slot, card 0, request 0xd.
    pub fn load(&mut self, slot: i32) {
        self.save_card = 0;
        self.card.slot = slot;
        self.request(0xd);
    }
}

/// `memcard_Save`'s capture (module docs): clock, landmarks, the current level's map mask.
pub fn capture(gs: &mut GameState, cap: &SaveCapture) {
    gs.global.save_clock = cap.clock;
    gs.capture_landmarks(gs.global.level, &cap.base, &cap.hooks);
    if let (Some(m), Some(l)) = (cap.map_mask, usize::try_from(gs.global.level).ok().and_then(|l| gs.levels.get_mut(l))) { l.map_mask = m; }
}

/// `RestoreData` of one section read from the card (a section that does not even decode: 1 error, nothing copied).
fn restore(gs: &mut GameState, scope: Scope, bytes: &[u8]) -> u32 {
    match Section::decode(bytes) {
        Ok(s) => gs.restore_section(scope, &s),
        Err(_) => 1,
    }
}

fn header(b: &[u8]) -> [u32; 2] {
    let w = |o: usize| b.get(o..o + 4).map_or(0, |x| u32::from_le_bytes(x.try_into().unwrap()));
    [w(0), w(4)]
}

/// `sceCdCLOCK` (BCD) of a civil date and time: [0] stat 0, second, minute, hour, pad 0, day, month, year % 100.
pub fn bcd_clock_civil(year: u64, month: u64, day: u64, hour: u64, minute: u64, second: u64) -> [u8; 8] {
    let bcd = |v: u64| (((v / 10) % 10) << 4 | (v % 10)) as u8;
    [0, bcd(second), bcd(minute), bcd(hour), 0, bcd(day), bcd(month), bcd(year % 100)]
}

/// `sceCdCLOCK` (BCD) of a Unix time in seconds, in UTC (the engine passes the host's local time instead when it
/// can, as the game's JST → local conversion 0x12d6d8 gives the console's local time: rc-engine `saves::clock_now`).
pub fn bcd_clock(unix: u64) -> [u8; 8] {
    let days = unix / 86400;
    let secs = unix % 86400;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + (m <= 2) as i64;
    bcd_clock_civil(y as u64, m as u64, d as u64, secs / 3600, secs / 60 % 60, secs % 60)
}

/// The save root of the native card: `RC_SAVE_DIR=<path>` (`0` or empty: no card), else `configured` (the engine's
/// settings key or the folder beside the executable), else `<config base>/rerac/memcard` next to the port settings
/// (`base` = the per-OS config folder the engine resolves); None = no card.
pub fn save_root(configured: Option<PathBuf>, base: Option<&Path>) -> Option<PathBuf> {
    if let Some(v) = std::env::var_os("RC_SAVE_DIR") {
        return (!v.is_empty() && v != "0").then(|| PathBuf::from(v));
    }
    configured.or_else(|| base.map(|b| b.join("rerac").join("memcard")))
}

impl MemCard {
    /// Set by status 22 when an auto-save failed (`mode_freezeInit(3, 0)`); the caller opens the dialog.
    pub fn take_freeze_request(&mut self) -> bool { std::mem::take(&mut self.freeze_request) }
}


#[cfg(test)]
mod clock_tests {
    use super::*;

    /// A real save's stamp (made 2026-10-15 20:09:48 at UTC−4): the UTC form the port wrote, and the local one.
    #[test]
    fn bcd_clock_utc_and_civil() {
        assert_eq!(bcd_clock(1_792_109_388), [0, 0x48, 0x09, 0x00, 0, 0x16, 0x10, 0x26]);
        assert_eq!(bcd_clock_civil(2026, 10, 15, 20, 9, 48), [0, 0x48, 0x09, 0x20, 0, 0x15, 0x10, 0x26]);
        assert_eq!(bcd_clock(0), [0, 0, 0, 0, 0, 0x01, 0x01, 0x70]);
    }
}
