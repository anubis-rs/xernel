#[derive(Debug)]
pub enum Error {
    VNodeNotFound,
    NotADirectory,
    IsADirectory,
    NoSpace,
    NotEmpty,
    EntryNotFound,
    MountPointNotFound,
    FileSystemNotFound,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
