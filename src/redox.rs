extern crate syscall;

use std::fs;
use std::io;
use std::os::unix::prelude::*;
use std::path::Path;

use FileTime;

pub fn set_file_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>) -> io::Result<()> {
    let (atime, mtime) = match (atime, mtime) {
        (Some(atime), Some(mtime)) => (atime, mtime),
        (None, None) => return Ok(()),
        _ => unimplemented!("Must set both atime and mtime on Redox"),
    };

    let fd = syscall::open(p.as_os_str().as_bytes(), 0)
        .map_err(|err| io::Error::from_raw_os_error(err.errno))?;
    set_file_times_redox(fd, atime, mtime)
}

pub fn set_symlink_file_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>) -> io::Result<()> {
    let (atime, mtime) = match (atime, mtime) {
        (Some(atime), Some(mtime)) => (atime, mtime),
        (None, None) => return Ok(()),
        _ => unimplemented!("Must set both atime and mtime on Redox"),
    };

    let fd = syscall::open(p.as_os_str().as_bytes(), syscall::O_NOFOLLOW)
        .map_err(|err| io::Error::from_raw_os_error(err.errno))?;
    set_file_times_redox(fd, atime, mtime)
}

fn set_file_times_redox(fd: usize, atime: FileTime, mtime: FileTime) -> io::Result<()> {
    use self::syscall::TimeSpec;

    fn to_timespec(ft: &FileTime) -> TimeSpec {
        TimeSpec {
            tv_sec: ft.seconds(),
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

pub fn from_creation_time(_meta: &fs::Metadata) -> Option<FileTime> {
    None
}
