//! On Linux we try to use the more accurate `utimensat` syscall but this isn't
//! always available so we also fall back to `utimes` if we couldn't find
//! `utimensat` at runtime.

use std::ffi::CString;
use std::ptr;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{Ordering::SeqCst, AtomicBool};
use std::os::unix::prelude::*;

use FileTime;

pub fn set_file_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>) -> io::Result<()> {
    set_times(p, atime, mtime, false)
}

pub fn set_file_handle_times(
    f: &fs::File,
    atime: Option<FileTime>,
    mtime: Option<FileTime>,
) -> io::Result<()> {
    // Attempt to use the `utimensat` syscall, but if it's not supported by the
    // current kernel then fall back to an older syscall.
    static INVALID: AtomicBool = AtomicBool::new(false);
    if !INVALID.load(SeqCst) {
        let times = [super::to_timespec(&atime), super::to_timespec(&mtime)];
        let rc = unsafe {
            libc::syscall(
                libc::SYS_utimensat,
                f.as_raw_fd(),
                ptr::null::<libc::c_char>(),
                times.as_ptr(),
                0,
            )
        };
        if rc == 0 {
            return Ok(())
        }
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ENOSYS) {
            INVALID.store(true, SeqCst);
        } else  {
            return Err(err)
        }
    }

    // Safe to unwrap atime and mtime -- without the utimensat feature enabled,
    // the public API only exposes methods that intialize atime and mtime as
    // Some.
    super::futimes(f, atime.unwrap(), mtime.unwrap(), libc::futimes)
}

pub fn set_symlink_file_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>) -> io::Result<()> {
    set_times(p, atime, mtime, true)
}

fn set_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>, symlink: bool) -> io::Result<()> {
    let flags = if symlink { libc::AT_SYMLINK_NOFOLLOW } else { 0 };

    // Same as the `if` statement above.
    static INVALID: AtomicBool = AtomicBool::new(false);
    if !INVALID.load(SeqCst) {
        let p = CString::new(p.as_os_str().as_bytes())?;
        let times = [super::to_timespec(&atime), super::to_timespec(&mtime)];
        let rc = unsafe {
            libc::syscall(
                libc::SYS_utimensat,
                libc::AT_FDCWD,
                p.as_ptr(),
                times.as_ptr(),
                flags,
            )
        };
        if rc == 0 {
            return Ok(())
        }
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ENOSYS) {
            INVALID.store(true, SeqCst);
        } else  {
            return Err(err)
        }
    }

    let utimes = if symlink { libc::lutimes } else { libc::utimes };

    // Safe to unwrap atime and mtime -- without the utimensat feature enabled, the public API
    // only exposes methods that intialize atime and mtime as Some.
    super::utimes(p, atime.unwrap(), mtime.unwrap(), utimes)
}
