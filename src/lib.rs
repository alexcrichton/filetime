//! Timestamps for files in Rust
//!
//! This library provides platform-agnostic inspection of the various timestamps
//! present in the standard `fs::Metadata` structure.
//!
//! # Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! filetime = "0.1"
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use std::fs;
//! use filetime::FileTime;
//!
//! let metadata = fs::metadata("foo.txt").unwrap();
//!
//! let mtime = FileTime::from_last_modification_time(&metadata);
//! println!("{}", mtime);
//!
//! let atime = FileTime::from_last_access_time(&metadata);
//! assert!(mtime < atime);
//!
//! // Inspect values that can be interpreted across platforms
//! println!("{}", mtime.seconds_relative_to_1970());
//! println!("{}", mtime.nanoseconds());
//!
//! // Print the platform-specific value of seconds
//! println!("{}", mtime.seconds());
//! ```

extern crate libc;

#[cfg(target_os = "redox")]
extern crate syscall;

#[cfg(windows)]
extern crate winapi;

#[cfg(any(unix, target_os = "redox"))] use std::os::unix::prelude::*;

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

/// A helper structure to represent a timestamp for a file.
///
/// The actual value contined within is platform-specific and does not have the
/// same meaning across platforms, but comparisons and stringification can be
/// significant among the same platform.
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone, Hash)]
pub struct FileTime {
    seconds: u64,
    nanos: u32,
}

impl FileTime {
    /// Creates a new timestamp representing a 0 time.
    ///
    /// Useful for creating the base of a cmp::max chain of times.
    pub fn zero() -> FileTime {
        FileTime { seconds: 0, nanos: 0 }
    }

    /// Creates a new instance of `FileTime` with a number of seconds and
    /// nanoseconds relative to January 1, 1970.
    ///
    /// Note that this is typically the relative point that Unix time stamps are
    /// from, but on Windows the native time stamp is relative to January 1,
    /// 1601 so the return value of `seconds` from the returned `FileTime`
    /// instance may not be the same as that passed in.
    pub fn from_seconds_since_1970(seconds: u64, nanos: u32) -> FileTime {
        FileTime {
            seconds: seconds + if cfg!(windows) {11644473600} else {0},
            nanos: nanos,
        }
    }

    /// Creates a new timestamp from the last modification time listed in the
    /// specified metadata.
    ///
    /// The returned value corresponds to the `mtime` field of `stat` on Unix
    /// platforms and the `ftLastWriteTime` field on Windows platforms.
    pub fn from_last_modification_time(meta: &fs::Metadata) -> FileTime {
        #[cfg(any(unix, target_os = "redox"))]
        fn imp(meta: &fs::Metadata) -> FileTime {
            FileTime::from_os_repr(meta.mtime() as u64, meta.mtime_nsec() as u32)
        }
        #[cfg(windows)]
        fn imp(meta: &fs::Metadata) -> FileTime {
            FileTime::from_os_repr(meta.last_write_time())
        }
        imp(meta)
    }

    /// Creates a new timestamp from the last access time listed in the
    /// specified metadata.
    ///
    /// The returned value corresponds to the `atime` field of `stat` on Unix
    /// platforms and the `ftLastAccessTime` field on Windows platforms.
    pub fn from_last_access_time(meta: &fs::Metadata) -> FileTime {
        #[cfg(any(unix, target_os = "redox"))]
        fn imp(meta: &fs::Metadata) -> FileTime {
            FileTime::from_os_repr(meta.atime() as u64, meta.atime_nsec() as u32)
        }
        #[cfg(windows)]
        fn imp(meta: &fs::Metadata) -> FileTime {
            FileTime::from_os_repr(meta.last_access_time())
        }
        imp(meta)
    }

    /// Creates a new timestamp from the creation time listed in the specified
    /// metadata.
    ///
    /// The returned value corresponds to the `birthtime` field of `stat` on
    /// Unix platforms and the `ftCreationTime` field on Windows platforms. Note
    /// that not all Unix platforms have this field available and may return
    /// `None` in some circumstances.
    pub fn from_creation_time(meta: &fs::Metadata) -> Option<FileTime> {
        macro_rules! birthtim {
            ($(($e:expr, $i:ident)),*) => {
                #[cfg(any($(target_os = $e),*))]
                fn imp(meta: &fs::Metadata) -> Option<FileTime> {
                    $(
                        #[cfg(target_os = $e)]
                        use std::os::$i::fs::MetadataExt;
                    )*
                    let raw = meta.as_raw_stat();
                    Some(FileTime::from_os_repr(raw.st_birthtime as u64,
                                                raw.st_birthtime_nsec as u32))
                }

                #[cfg(all(not(windows),
                          $(not(target_os = $e)),*))]
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

        #[cfg(windows)]
        fn imp(meta: &fs::Metadata) -> Option<FileTime> {
            Some(FileTime::from_os_repr(meta.last_access_time()))
        }
        imp(meta)
    }

    #[cfg(windows)]
    fn from_os_repr(time: u64) -> FileTime {
        // Windows write times are in 100ns intervals, so do a little math to
        // get it into the right representation.
        FileTime {
            seconds: time / (1_000_000_000 / 100),
            nanos: ((time % (1_000_000_000 / 100)) * 100) as u32,
        }
    }

    #[cfg(any(unix, target_os = "redox"))]
    fn from_os_repr(seconds: u64, nanos: u32) -> FileTime {
        FileTime { seconds: seconds, nanos: nanos }
    }

    /// Returns the whole number of seconds represented by this timestamp.
    ///
    /// Note that this value's meaning is **platform specific**. On Unix
    /// platform time stamps are typically relative to January 1, 1970, but on
    /// Windows platforms time stamps are relative to January 1, 1601.
    pub fn seconds(&self) -> u64 { self.seconds }

    /// Returns the whole number of seconds represented by this timestamp,
    /// relative to the Unix epoch start of January 1, 1970.
    ///
    /// Note that this does not return the same value as `seconds` for Windows
    /// platforms as seconds are relative to a different date there.
    pub fn seconds_relative_to_1970(&self) -> u64 {
        self.seconds - if cfg!(windows) {11644473600} else {0}
    }

    /// Returns the nanosecond precision of this timestamp.
    ///
    /// The returned value is always less than one billion and represents a
    /// portion of a second forward from the seconds returned by the `seconds`
    /// method.
    pub fn nanoseconds(&self) -> u32 { self.nanos }
}

impl fmt::Display for FileTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{:09}s", self.seconds, self.nanos)
    }
}

/// Set the last access and modification times for a file on the filesystem.
///
/// This function will set the `atime` and `mtime` metadata fields for a file
/// on the local filesystem, returning any error encountered.
pub fn set_file_times<P>(p: P, atime: FileTime, mtime: FileTime)
                         -> io::Result<()> where P: AsRef<Path> {
    set_file_times_(p.as_ref(), atime, mtime)
}

/// Set the last access and modification times for a file on the filesystem.
/// This function does not follow symlink.
///
/// This function will set the `atime` and `mtime` metadata fields for a file
/// on the local filesystem, returning any error encountered.
pub fn set_symlink_file_times<P>(p: P, atime: FileTime, mtime: FileTime)
                                 -> io::Result<()> where P: AsRef<Path> {
    set_symlink_file_times_(p.as_ref(), atime, mtime)
}

use self::imp::{set_file_times_, set_symlink_file_times_};

// utimes based implementation: More generally available, but provides
// only ms-grain precision.
#[cfg(any(target_os = "macos",
          target_os = "ios",
          target_os = "freebsd",
          target_os = "dragonfly",
          target_os = "openbsd",
          target_os = "netbsd",
          target_os = "bitrig",
          target_os = "solaris",
          target_os = "haiku"))]
mod imp {
    use std::io;
    use std::os::unix::prelude::*;
    use std::path::Path;
    use libc::{c_char, c_int, timeval};

    use super::FileTime;

    pub(super) fn set_file_times_(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
        use libc::utimes;
        fn set_time(filename: *const c_char, times: *const timeval) -> c_int {
            unsafe {
                utimes(filename, times)
            }
        }
        return set_file_times_u(p, atime, mtime, set_time);
    }

    pub(super) fn set_symlink_file_times_(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
        use libc::lutimes;
        fn set_time(filename: *const c_char, times: *const timeval) -> c_int {
            unsafe {
                lutimes(filename, times)
            }
        }
        set_file_times_u(p, atime, mtime, set_time)
    }

    fn set_file_times_u<ST>(p: &Path, atime: FileTime, mtime: FileTime, utimes: ST) -> io::Result<()>
        where ST: Fn(*const c_char, *const timeval) -> c_int
    {
        use std::ffi::CString;
        use libc::{timeval, time_t, suseconds_t};

        let times = [to_timeval(&atime), to_timeval(&mtime)];
        let p = try!(CString::new(p.as_os_str().as_bytes()));
        return if utimes(p.as_ptr() as *const _, times.as_ptr()) == 0 {
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
}

// utimensat based implementation: Only available on notbsd unix, but
// provides ns-grain precision.
#[cfg(any(target_os = "linux",
          target_os = "android",
          target_os = "emscripten",
          target_os = "fuchsia",
          target_env = "uclibc"))]
mod imp {
    use std::io;
    use std::os::unix::prelude::*;
    use std::path::Path;
    use libc::{c_char, c_int, timespec};

    use super::FileTime;

    pub(super) fn set_file_times_(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
        use libc::{utimensat, AT_FDCWD};
        fn set_time(filename: *const c_char, times: *const timespec) -> c_int {
            unsafe {
                // Passing AT_FDCWD interprets a relative filename from
                // working directory, analogous to behavior of `utimes`.
                utimensat(AT_FDCWD, filename, times, 0)
            }
        }
        return set_file_times_ns(p, atime, mtime, set_time);
    }

    pub(super) fn set_symlink_file_times_(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
        use libc::{utimensat, AT_FDCWD, AT_SYMLINK_NOFOLLOW};
        fn set_time(filename: *const c_char, times: *const timespec) -> c_int {
            unsafe {
                // Passing AT_FDCWD interprets a relative filename from
                // working directory, analogous to behavior of `utimes`.
                utimensat(AT_FDCWD, filename, times, AT_SYMLINK_NOFOLLOW)
            }
        }
        set_file_times_ns(p, atime, mtime, set_time)
    }

    fn set_file_times_ns<ST>(p: &Path, atime: FileTime, mtime: FileTime, utimes_ns: ST) -> io::Result<()>
        where ST: Fn(*const c_char, *const timespec) -> c_int
    {
        use std::ffi::CString;
        use libc::{timespec, time_t, c_long};

        let times = [to_timespec(&atime), to_timespec(&mtime)];
        let p = try!(CString::new(p.as_os_str().as_bytes()));
        return if utimes_ns(p.as_ptr() as *const _, times.as_ptr()) == 0 {
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
}

// Redox implementation: uses syscalls directly
#[cfg(target_os = "redox")]
mod imp {
    use std::io;
    use std::os::unix::prelude::*;
    use std::path::Path;

    use super::FileTime;

    pub(super) fn set_file_times_(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
        let fd = syscall::open(p.as_os_str().as_bytes(), 0)
            .map_err(|err| io::Error::from_raw_os_error(err.errno))?;
        set_file_times_redox(fd, atime, mtime)
    }

    pub(super) fn set_symlink_file_times_(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
        let fd = syscall::open(p.as_os_str().as_bytes(), syscall::O_NOFOLLOW)
            .map_err(|err| io::Error::from_raw_os_error(err.errno))?;
        set_file_times_redox(fd, atime, mtime)
    }

    fn set_file_times_redox(fd: usize, atime: FileTime, mtime: FileTime) -> io::Result<()> {
        use syscall::TimeSpec;

        fn to_timespec(ft: &FileTime) -> TimeSpec {
            syscall::TimeSpec {
                tv_sec: ft.seconds() as i64,
                tv_nsec: ft.nanoseconds() as i32
            }
        }

        let times = [to_timespec(&atime), to_timespec(&mtime)];
        let res = syscall::futimens(fd, &times);
        let _ = syscall::close(fd);
        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(io::Error::from_raw_os_error(err.errno))
        }
    }
}

// Windows implementation: has an entirely different API.
#[cfg(windows)]
#[allow(bad_style)]
mod imp {
    use std::io;
    use std::path::Path;
    use std::os::windows::prelude::*;
    use std::fs::OpenOptions;

    use super::FileTime;

    pub(super) fn set_file_times_(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
        set_file_times_w(p, atime, mtime, OpenOptions::new())
    }

    pub(super) fn set_symlink_file_times_(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
        use std::os::windows::fs::OpenOptionsExt;
        use winapi::winbase::FILE_FLAG_OPEN_REPARSE_POINT;

        let mut options = OpenOptions::new();
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        set_file_times_w(p, atime, mtime, options)
    }

    fn set_file_times_w(p: &Path, atime: FileTime, mtime: FileTime, mut options: OpenOptions) -> io::Result<()> {
        type BOOL = i32;
        type HANDLE = *mut u8;
        type DWORD = u32;
        #[repr(C)]
        struct FILETIME {
            dwLowDateTime: u32,
            dwHighDateTime: u32,
        }
        extern "system" {
            fn SetFileTime(hFile: HANDLE,
                           lpCreationTime: *const FILETIME,
                           lpLastAccessTime: *const FILETIME,
                           lpLastWriteTime: *const FILETIME) -> BOOL;
        }

        let f = try!(options.write(true).open(p));
        let atime = to_filetime(&atime);
        let mtime = to_filetime(&mtime);
        return unsafe {
            let ret = SetFileTime(f.as_raw_handle() as *mut _,
                                  0 as *const _,
                                  &atime, &mtime);
            if ret != 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        };

        fn to_filetime(ft: &FileTime) -> FILETIME {
            let intervals = ft.seconds() * (1_000_000_000 / 100) +
                ((ft.nanoseconds() as u64) / 100);
            FILETIME {
                dwLowDateTime: intervals as DWORD,
                dwHighDateTime: (intervals >> 32) as DWORD,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use std::io;
    use std::path::Path;
    use std::fs::{self, File};
    use self::tempdir::TempDir;
    use super::{FileTime, set_file_times, set_symlink_file_times};

    #[cfg(unix)]
    fn make_symlink<P,Q>(src: P, dst: Q) -> io::Result<()>
        where P: AsRef<Path>,
              Q: AsRef<Path>,
    {
        use std::os::unix::fs::symlink;
        symlink(src, dst)
    }

    #[cfg(windows)]
    fn make_symlink<P,Q>(src: P, dst: Q) -> io::Result<()>
        where P: AsRef<Path>,
              Q: AsRef<Path>,
    {
        use std::os::windows::fs::symlink_file;
        symlink_file(src, dst)
    }

    #[test]
    fn set_file_times_test() {
        let td = TempDir::new("filetime").unwrap();
        let path = td.path().join("foo.txt");
        File::create(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        let atime = FileTime::from_last_access_time(&metadata);
        set_file_times(&path, atime, mtime).unwrap();

        let new_mtime = FileTime::from_seconds_since_1970(10_000, 0);
        set_file_times(&path, atime, new_mtime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime);

        let spath = td.path().join("bar.txt");
        make_symlink(&path, &spath).unwrap();
        let metadata = fs::symlink_metadata(&spath).unwrap();
        let smtime = FileTime::from_last_modification_time(&metadata);

        set_file_times(&spath, atime, mtime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let cur_mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, cur_mtime);

        let metadata = fs::symlink_metadata(&spath).unwrap();
        let cur_mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(smtime, cur_mtime);

        set_file_times(&spath, atime, new_mtime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime);

        let metadata = fs::symlink_metadata(&spath).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, smtime);
    }

    #[test]
    fn set_symlink_file_times_test() {
        let td = TempDir::new("filetime").unwrap();
        let path = td.path().join("foo.txt");
        File::create(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        let atime = FileTime::from_last_access_time(&metadata);
        set_symlink_file_times(&path, atime, mtime).unwrap();

        let new_mtime = FileTime::from_seconds_since_1970(10_000, 0);
        set_symlink_file_times(&path, atime, new_mtime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime);

        let spath = td.path().join("bar.txt");
        make_symlink(&path, &spath).unwrap();

        let metadata = fs::symlink_metadata(&spath).unwrap();
        let smtime = FileTime::from_last_modification_time(&metadata);
        let satime = FileTime::from_last_access_time(&metadata);
        set_symlink_file_times(&spath, smtime, satime).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime);

        let new_smtime = FileTime::from_seconds_since_1970(20_000, 0);
        set_symlink_file_times(&spath, atime, new_smtime).unwrap();

        let metadata = fs::metadata(&spath).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_mtime);

        let metadata = fs::symlink_metadata(&spath).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime, new_smtime);
    }
}
