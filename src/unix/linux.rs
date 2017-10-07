//! On Linux we try to use the more accurate `utimensat` syscall but this isn't
//! always available so we also fall back to `utimes` if we couldn't find
//! `utimensat` at runtime.

use std::io;
use std::mem;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

use FileTime;
use super::libc::{self, c_int, c_char, timespec};

pub fn set_file_times(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
    match utimensat() {
        Some(f) => super::utimensat(p, atime, mtime, f, 0),
        None => super::utimes(p, atime, mtime, libc::utimes),
    }
}

pub fn set_symlink_file_times(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
    match utimensat() {
        Some(f) => super::utimensat(p, atime, mtime, f, libc::AT_SYMLINK_NOFOLLOW),
        None => super::utimes(p, atime, mtime, libc::lutimes),
    }
}

fn utimensat() -> Option<unsafe extern fn(c_int, *const c_char, *const timespec, c_int) -> c_int> {
    static ADDR: AtomicUsize = ATOMIC_USIZE_INIT;
    unsafe {
        match ADDR.load(Ordering::SeqCst) {
            0 => {}
            1 => return None,
            n => return Some(mem::transmute(n)),
        }
        let name = b"utimensat\0";
        let sym = libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr() as *const _);
        let (val, ret) = if sym.is_null() {
            (1, None)
        } else {
            (sym as usize, Some(mem::transmute(sym)))
        };
        ADDR.store(val, Ordering::SeqCst);
        return ret
    }
}
