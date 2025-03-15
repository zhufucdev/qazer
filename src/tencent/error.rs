use std::fmt::{Display, Formatter};
use std::io;

#[derive(Debug)]
pub enum Error {
    TokenExpired,
    Http(reqwest::Error),
    Parse(reqwest::Error),
    IO(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::TokenExpired => write!(f, "token expired"),
            Error::Http(e) => write!(f, "http: {}", e),
            Error::Parse(e) => write!(f, "parser: {}", e),
            Error::IO(e) => write!(f, "io: {}", e),
        }
    }
}
