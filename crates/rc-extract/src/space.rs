//! Free disk space at a path, through the platform C library (no crate dependency). `None` where unsupported;
//! the extractor then skips the check (a full disk still fails the write with code 31).

use std::path::Path;

/// Bytes available to this user on the volume holding `path` (or its nearest existing ancestor).
pub fn available_bytes(path: &Path) -> Option<u64> {
    let mut p = path;
    while !p.exists() { p = p.parent()?; }
    let p = if p.as_os_str().is_empty() { Path::new(".") } else { p };
    imp::available(p)
}

#[cfg(any(all(target_os = "macos", any(target_arch = "aarch64", target_arch = "x86_64")), all(target_os = "ios", target_arch = "aarch64")))]
mod imp {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    /// `struct statfs` (64-bit inode layout, the only one on arm64; `statfs$INODE64` on x86_64), from <sys/mount.h>.
    #[repr(C)]
    #[allow(dead_code)]
    struct StatFs {
        f_bsize: u32,
        f_iosize: i32,
        f_blocks: u64,
        f_bfree: u64,
        f_bavail: u64,
        f_files: u64,
        f_ffree: u64,
        f_fsid: [i32; 2],
        f_owner: u32,
        f_type: u32,
        f_flags: u32,
        f_fssubtype: u32,
        f_fstypename: [u8; 16],
        f_mntonname: [u8; 1024],
        f_mntfromname: [u8; 1024],
        f_flags_ext: u32,
        f_reserved: [u32; 7],
    }

    extern "C" {
        #[cfg_attr(target_arch = "x86_64", link_name = "statfs$INODE64")]
        fn statfs(path: *const std::ffi::c_char, buf: *mut StatFs) -> std::ffi::c_int;
    }

    pub(super) fn available(p: &std::path::Path) -> Option<u64> {
        let c = CString::new(p.as_os_str().as_bytes()).ok()?;
        let mut s = std::mem::MaybeUninit::<StatFs>::zeroed();
        // SAFETY: `c` is NUL-terminated and `s` is a writable, correctly laid-out `struct statfs`.
        let r = unsafe { statfs(c.as_ptr(), s.as_mut_ptr()) };
        if r != 0 { return None; }
        // SAFETY: statfs succeeded and filled the struct (it was zero-initialised anyway).
        let s = unsafe { s.assume_init() };
        Some(s.f_bavail.saturating_mul(s.f_bsize as u64))
    }
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
mod imp {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    /// `struct statvfs` on 64-bit Linux (glibc and musl), from <sys/statvfs.h>.
    #[repr(C)]
    #[allow(dead_code)]
    struct StatVfs {
        f_bsize: u64,
        f_frsize: u64,
        f_blocks: u64,
        f_bfree: u64,
        f_bavail: u64,
        f_files: u64,
        f_ffree: u64,
        f_favail: u64,
        f_fsid: u64,
        f_flag: u64,
        f_namemax: u64,
        spare: [i32; 6],
    }

    extern "C" {
        fn statvfs(path: *const std::ffi::c_char, buf: *mut StatVfs) -> std::ffi::c_int;
    }

    pub(super) fn available(p: &std::path::Path) -> Option<u64> {
        let c = CString::new(p.as_os_str().as_bytes()).ok()?;
        let mut s = std::mem::MaybeUninit::<StatVfs>::zeroed();
        // SAFETY: `c` is NUL-terminated and `s` is a writable, correctly laid-out `struct statvfs`.
        let r = unsafe { statvfs(c.as_ptr(), s.as_mut_ptr()) };
        if r != 0 { return None; }
        // SAFETY: statvfs succeeded and filled the struct.
        let s = unsafe { s.assume_init() };
        Some(s.f_bavail.saturating_mul(s.f_frsize.max(1)))
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(dir: *const u16, avail: *mut u64, total: *mut u64, free: *mut u64) -> i32;
    }

    pub(super) fn available(p: &std::path::Path) -> Option<u64> {
        let wide: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
        // SAFETY: `wide` is NUL-terminated UTF-16; the out pointers are valid u64s.
        let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, &mut total, &mut free) };
        (ok != 0).then_some(avail)
    }
}

#[cfg(not(any(
    all(target_os = "macos", any(target_arch = "aarch64", target_arch = "x86_64")),
    all(target_os = "ios", target_arch = "aarch64"),
    all(target_os = "linux", target_pointer_width = "64"),
    windows
)))]
mod imp {
    pub(super) fn available(_: &std::path::Path) -> Option<u64> { None }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reports_something_plausible_for_the_temp_dir() {
        let tmp = std::env::temp_dir();
        if cfg!(any(target_os = "macos", target_os = "linux", windows)) {
            let n = super::available_bytes(&tmp.join("does/not/exist/yet")).expect("free space");
            assert!(n > 0);
            // Same volume, same answer (within a few MiB of concurrent writes).
            let m = super::available_bytes(&tmp).unwrap();
            assert!(n.abs_diff(m) < 256 << 20, "{n} vs {m}");
        }
    }
}
