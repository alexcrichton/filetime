//! Timestamps for files in Rust
//!
//! This library provides platform-agnostic inspection of the various timestamps
//! present in the standard `fs::Metadata` structure.
//!
//! # Installation
//!
//! Add this to you `Cargo.toml`:
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

#[cfg(unix)] use std::os::unix::prelude::*;
#[cfg(windows)] use std::os::windows::prelude::*;

use std::fmt;
use std::fs;

/// A helper structure to represent a timestamp for a file.
///
/// The actual value contined within is platform-specific and does not have the
/// same meaning across platforms, but comparisons and stringification can be
/// significant among the same platform.
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone)]
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

    /// Creates a new timestamp from the last modification time listed in the
    /// specified metadata.
    ///
    /// The returned value corresponds to the `mtime` field of `stat` on Unix
    /// platforms and the `ftLastWriteTime` field on Windows platforms.
    pub fn from_last_modification_time(meta: &fs::Metadata) -> FileTime {
        #[cfg(unix)]
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
        #[cfg(unix)]
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
            ("openbsd", openbsd),
            ("dragonfly", dragonfly)
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

    #[cfg(unix)]
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
