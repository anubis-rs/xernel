#[derive(Debug)]
pub enum Error {
    VNodeNotFound,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
