#[derive(Debug)]
pub enum Error {
    NodeNotFound,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
