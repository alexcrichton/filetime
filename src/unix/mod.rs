extern crate libc;

use std::ffi::CString;
use std::fs;
use std::io;
use std::os::unix::prelude::*;
use std::path::Path;

use self::libc::{c_int, c_char, timeval, time_t, suseconds_t};
use self::libc::{timespec};

use FileTime;

cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod linux;
        pub use self::linux::*;
    } else if #[cfg(any(target_os = "android",
                        target_os = "solaris",
                        target_os = "emscripten",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "openbsd"))] {
        mod utimensat;
        pub use self::utimensat::*;
    } else {
        mod utimes;
        pub use self::utimes::*;
    }
}

#[allow(dead_code)]
fn utimes(p: &Path,
          atime: Option<FileTime>,
          mtime: Option<FileTime>,
          utimes: unsafe extern fn(*const c_char, *const timeval) -> c_int)
    -> io::Result<()>
{
    let atime = if let Some(atime) = atime {
        atime
    } else {
        let meta = p.metadata().expect("file should have metadata");
        from_last_access_time(&meta)
    };

    let mtime = if let Some(mtime) = mtime {
        mtime
    } else {
        let meta = p.metadata().expect("file should have metadata");
        from_last_modification_time(&meta)
    };

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

#[allow(dead_code)]
fn utimensat(p: &Path,
             atime: Option<FileTime>,
             mtime: Option<FileTime>,
             f: unsafe extern fn(c_int, *const c_char, *const timespec, c_int) -> c_int,
             flags: c_int)
    -> io::Result<()>
{
    let times = [to_timespec(&atime), to_timespec(&mtime)];
    let p = try!(CString::new(p.as_os_str().as_bytes()));
    let rc = unsafe {
        f(libc::AT_FDCWD, p.as_ptr() as *const _, times.as_ptr(), flags)
    };
    return if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    };

    fn to_timespec(ft: &Option<FileTime>) -> timespec {
        const UTIME_OMIT: i64 = 1073741822;

        if let &Some(ft) = ft {
            timespec {
                tv_sec: ft.seconds() as time_t,
                tv_nsec: ft.nanoseconds() as _,
            }
        } else {
            timespec {
                tv_sec: 0,
                tv_nsec: UTIME_OMIT,
            }
        }
    }
}

pub fn from_last_modification_time(meta: &fs::Metadata) -> FileTime {
    FileTime {
        seconds: meta.mtime(),
        nanos: meta.mtime_nsec() as u32,
    }
}

pub fn from_last_access_time(meta: &fs::Metadata) -> FileTime {
    FileTime {
        seconds: meta.atime(),
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
                    seconds: meta.st_birthtime(),
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
