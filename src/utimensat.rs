extern crate libc;

use std::ffi::CString;
use std::fs;
use std::io;
use std::os::unix::prelude::*;
use std::path::Path;

use self::libc::{c_int, timespec, time_t, c_long};

use FileTime;

pub fn set_file_times(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
    set_file_times_u(p, atime, mtime, 0)
}

pub fn set_symlink_file_times(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
   set_file_times_u(p, atime, mtime, libc::AT_SYMLINK_NOFOLLOW)
}

fn set_file_times_u(p: &Path,
                    atime: FileTime,
                    mtime: FileTime,
                    flags: c_int)
    -> io::Result<()>
{
    let times = [to_timespec(&atime), to_timespec(&mtime)];
    let p = try!(CString::new(p.as_os_str().as_bytes()));
    let rc = unsafe {
        libc::utimensat(libc::AT_FDCWD,
                        p.as_ptr() as *const _,
                        times.as_ptr(),
                        flags)
    };
    return if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    };

    fn to_timespec(ft: &FileTime) -> timespec {
        timespec {
            tv_sec: ft.seconds() as time_t,
            tv_nsec: ft.nanoseconds() as c_long,
        }
    }
}

pub fn from_last_modification_time(meta: &fs::Metadata) -> FileTime {
    FileTime {
        seconds: meta.mtime() as u64,
        nanos: meta.mtime_nsec() as u32,
    }
}

pub fn from_last_access_time(meta: &fs::Metadata) -> FileTime {
    FileTime {
        seconds: meta.atime() as u64,
        nanos: meta.atime_nsec() as u32,
    }
}

pub fn from_creation_time(_ta: &fs::Metadata) -> Option<FileTime> {
   None
}
