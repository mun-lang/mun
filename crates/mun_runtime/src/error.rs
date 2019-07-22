use std::convert::From;
use std::io;
use std::result::Result as StdResult;

use failure;
use notify;

#[derive(Debug)]
pub enum Error {
    Cargo(failure::Error),
    IO(io::Error),
    Watch(notify::Error),
}

impl From<failure::Error> for Error {
    fn from(error: failure::Error) -> Self {
        Error::Cargo(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IO(error)
    }
}

impl From<notify::Error> for Error {
    fn from(error: notify::Error) -> Self {
        Error::Watch(error)
    }
}

pub type Result<T> = StdResult<T, Error>;
