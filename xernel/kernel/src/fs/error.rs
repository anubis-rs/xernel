#[derive(Debug)]
pub enum Error {
    VNodeNotFound,
    NotADirectory,
    NoSpace,
    NotEmpty,
    EntryNotFound,
    MountPointNotFound,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
