use std::path::Path;
use std::io;

use FileTime;
use super::libc;

pub fn set_file_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>) -> io::Result<()> {
    // Safe to unwrap atime and mtime -- platforms which use the utimes module
    // only expose public API functions to set both at once
    super::utimes(p, atime.unwrap(), mtime.unwrap(), libc::utimes)
}

pub fn set_symlink_file_times(p: &Path, atime: Option<FileTime>, mtime: Option<FileTime>) -> io::Result<()> {
    // Safe to unwrap atime and mtime -- platforms which use the utimes module
    // only expose public API functions to set both at once
    super::utimes(p, atime.unwrap(), mtime.unwrap(), libc::lutimes)
}
