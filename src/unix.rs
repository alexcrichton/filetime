extern crate libc;

use std::ffi::CString;
use std::fs;
use std::io;
use std::os::unix::prelude::*;
use std::path::Path;

use self::libc::{c_int, c_char, timeval, time_t, suseconds_t};

use FileTime;

pub fn set_file_times(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
    set_file_times_u(p, atime, mtime, libc::utimes)
}

#[cfg(target_os = "android")]
pub fn set_symlink_file_times(_p: &Path, _atime: FileTime, _mtime: FileTime) -> io::Result<()> {
   Err(io::Error::new(io::ErrorKind::Other, "not supported on Android"))
}

#[cfg(not(target_os = "android"))]
pub fn set_symlink_file_times(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
   set_file_times_u(p, atime, mtime, libc::lutimes)
}

fn set_file_times_u(p: &Path,
                    atime: FileTime,
                    mtime: FileTime,
                    utimes: unsafe extern fn(*const c_char, *const timeval) -> c_int)
    -> io::Result<()>
{
    let times = [to_timeval(&atime), to_timeval(&mtime)];
    let p = try!(CString::new(p.as_os_str().as_bytes()));
    return if unsafe { utimes(p.as_ptr() as *const _, times.as_ptr()) == 0 } {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    };

    fn to_timeval(ft: &FileTime) -> timeval {
        timeval {
            tv_sec: ft.seconds() as time_t,
            tv_usec: (ft.nanoseconds() / 1000) as suseconds_t,
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

pub fn from_creation_time(meta: &fs::Metadata) -> Option<FileTime> {
    macro_rules! birthtim {
        ($(($e:expr, $i:ident)),*) => {
            #[cfg(any($(target_os = $e),*))]
            fn imp(meta: &fs::Metadata) -> Option<FileTime> {
                $(
                    #[cfg(target_os = $e)]
                    use std::os::$i::fs::MetadataExt;
                )*
                Some(FileTime {
                    seconds: meta.st_birthtime() as u64,
                    nanos: meta.st_birthtime_nsec() as u32,
                })
            }

            #[cfg(all($(not(target_os = $e)),*))]
            fn imp(_meta: &fs::Metadata) -> Option<FileTime> {
                None
            }
        }
    }

    birthtim! {
        ("bitrig", bitrig),
        ("freebsd", freebsd),
        ("ios", ios),
        ("macos", macos),
        ("openbsd", openbsd)
    }

    imp(meta)
}
