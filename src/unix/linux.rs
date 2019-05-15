//! On Linux we try to use the more accurate `utimensat` syscall but this isn't
//! always available so we also fall back to `utimes` if we couldn't find
//! `utimensat` at runtime.

use std::fs;
use std::io;
use std::mem;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT};

use FileTime;
use super::libc::{self, c_int, c_char, timespec};

pub fn set_file_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>) -> io::Result<()> {
    set_times(p, atime, mtime, false)
}

pub fn set_file_handle_times(
    f: &mut fs::File,
    atime: Option<FileTime>,
    mtime: Option<FileTime>,
) -> io::Result<()> {
    if cfg!(feature = "utimensat") {
         match futimens() {
            Some(func) => {
                super::futimens(f, atime, mtime, func)
            },
            None => {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Version of libc does not support the futimens syscall"
                ))
            }
        }
    } else {
        // Try to use the more-accurate `utimensat` when possible.
        static INVALID: AtomicBool = ATOMIC_BOOL_INIT;
        if !INVALID.load(Ordering::SeqCst) {
            if let Some(func) = futimens() {
                // Even when libc has `futimens`, the kernel may return `ENOSYS`,
                // and then we'll need to use the `utimes` fallback instead.
                match super::futimens(f, atime, mtime, func) {
                    Err(ref e) if e.raw_os_error() == Some(libc::ENOSYS) => {
                        INVALID.store(true, Ordering::SeqCst);
                    }
                    valid => return valid,
                }
            }
        }

        // Safe to unwrap atime and mtime -- without the utimensat feature enabled, the public API
        // only exposes methods that intialize atime and mtime as Some.
        super::futimes(f, atime.unwrap(), mtime.unwrap(), libc::futimes)
    }

}

pub fn set_symlink_file_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>) -> io::Result<()> {
    set_times(p, atime, mtime, true)
}

fn set_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>, symlink: bool) -> io::Result<()> {
    let flags = if symlink { libc::AT_SYMLINK_NOFOLLOW } else { 0 };

    if cfg!(feature = "utimensat") {
         match utimensat() {
            Some(f) => {
                super::utimensat(p, atime, mtime, f, flags)
            },
            None => {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Version of libc does not support the utimensat syscall"
                ))
            }
        }
    } else {
        // Try to use the more-accurate `utimensat` when possible.
        static INVALID: AtomicBool = ATOMIC_BOOL_INIT;
        if !INVALID.load(Ordering::SeqCst) {
            if let Some(f) = utimensat() {
                // Even when libc has `utimensat`, the kernel may return `ENOSYS`,
                // and then we'll need to use the `utimes` fallback instead.
                match super::utimensat(p, atime, mtime, f, flags) {
                    Err(ref e) if e.raw_os_error() == Some(libc::ENOSYS) => {
                        INVALID.store(true, Ordering::SeqCst);
                    }
                    valid => return valid,
                }
            }
        }

        let utimes = if symlink { libc::lutimes } else { libc::utimes };

        // Safe to unwrap atime and mtime -- without the utimensat feature enabled, the public API
        // only exposes methods that intialize atime and mtime as Some.
        super::utimes(p, atime.unwrap(), mtime.unwrap(), utimes)
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

fn futimens() -> Option<unsafe extern fn(c_int, *const timespec) -> c_int> {
    static ADDR: AtomicUsize = ATOMIC_USIZE_INIT;
    unsafe {
        match ADDR.load(Ordering::SeqCst) {
            0 => {}
            1 => return None,
            n => return Some(mem::transmute(n)),
        }
        let name = b"futimens\0";
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
