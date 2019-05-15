#![allow(bad_style)]

use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::windows::prelude::*;
use std::path::Path;

use FileTime;

pub fn set_file_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>) -> io::Result<()> {
    let mut f = OpenOptions::new().write(true).open(p)?;
    set_file_times_w(&mut f, atime, mtime)
}

pub fn set_file_handle_times(
    f: &mut File,
    atime: Option<FileTime>,
    mtime: Option<FileTime>,
) -> io::Result<()> {
    set_file_times_w(f, atime, mtime)
}

pub fn set_symlink_file_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>) -> io::Result<()> {
    let (atime, mtime) = match (atime, mtime) {
        (Some(atime), Some(mtime)) => (atime, mtime),
        (None, None) => return Ok(()),
        _ => unimplemented!("Must set both atime and mtime on Windows"),
    };

    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x00200000;

    let mut f = OpenOptions::new()
        .write(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(p)?;
    set_file_times_w(&mut f, Some(atime), Some(mtime))
}

fn set_file_times_w(
    f: &mut File,
    atime: Option<FileTime>,
    mtime: Option<FileTime>,
) -> io::Result<()> {
    type BOOL = i32;
    type HANDLE = *mut u8;
    type DWORD = u32;

    #[repr(C)]
    struct FILETIME {
        dwLowDateTime: u32,
        dwHighDateTime: u32,
    }

    extern "system" {
        fn SetFileTime(
            hFile: HANDLE,
            lpCreationTime: Option<& FILETIME>,
            lpLastAccessTime: Option<& FILETIME>,
            lpLastWriteTime: Option<& FILETIME>,
        ) -> BOOL;
    }

    let atime = atime.map(to_filetime);
    let mtime = mtime.map(to_filetime);
    return unsafe {
        let ret = SetFileTime(f.as_raw_handle() as *mut _, None, atime.as_ref(), mtime.as_ref());
        if ret != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    };

    fn to_filetime(ft: FileTime) -> FILETIME {
        let intervals = ft.seconds() * (1_000_000_000 / 100) + ((ft.nanoseconds() as i64) / 100);
        FILETIME {
            dwLowDateTime: intervals as DWORD,
            dwHighDateTime: (intervals >> 32) as DWORD,
        }
    }
}

pub fn from_last_modification_time(meta: &fs::Metadata) -> FileTime {
    from_intervals(meta.last_write_time())
}

pub fn from_last_access_time(meta: &fs::Metadata) -> FileTime {
    from_intervals(meta.last_access_time())
}

pub fn from_creation_time(meta: &fs::Metadata) -> Option<FileTime> {
    Some(from_intervals(meta.creation_time()))
}

fn from_intervals(ticks: u64) -> FileTime {
    // Windows write times are in 100ns intervals, so do a little math to
    // get it into the right representation.
    FileTime {
        seconds: (ticks / (1_000_000_000 / 100)) as i64,
        nanos: ((ticks % (1_000_000_000 / 100)) * 100) as u32,
    }
}
