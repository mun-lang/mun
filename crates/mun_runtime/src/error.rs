use std::convert::From;
use std::io;
use std::result::Result as StdResult;

use notify;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    Watcher(notify::Error),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IO(error)
    }
}

impl From<notify::Error> for Error {
    fn from(error: notify::Error) -> Self {
        Error::Watcher(error)
    }
}

pub type Result<T> = StdResult<T, Error>;
