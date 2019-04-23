#![allow(unused_variables)]
#![allow(bad_style)]

use std::path::Path;
use std::{fs, io};
use FileTime;

pub fn set_file_times(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Other, "Wasm not implemented"))
}

pub fn set_symlink_file_times(p: &Path, atime: FileTime, mtime: FileTime) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Other, "Wasm not implemented"))
}

pub fn from_last_modification_time(meta: &fs::Metadata) -> FileTime {
    unimplemented!()
}

pub fn from_last_access_time(meta: &fs::Metadata) -> FileTime {
    unimplemented!()
}

pub fn from_creation_time(meta: &fs::Metadata) -> Option<FileTime> {
    unimplemented!()
}
