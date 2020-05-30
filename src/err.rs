use std::string::ToString;
use std::{io, result};

use thiserror::Error;

pub type Result<T> = result::Result<T, Error>;

pub struct Error {
    msg: String,
    source: io::Error,
}

impl Error {
    #[inline]
    pub(crate) fn new(msg: String, source: io::Error) -> Self {
        Self { msg, source }
    }
}
