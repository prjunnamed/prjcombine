use std::io;
use nix;
use std::num;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    NixError(nix::Error),
    ParseError(String),
}

impl From<io::Error> for Error {
    fn from(x: io::Error) -> Error {
        Error::IoError(x)
    }
}

impl From<nix::Error> for Error {
    fn from(x: nix::Error) -> Error {
        Error::NixError(x)
    }
}

impl From<Error> for io::Error {
    fn from(x: Error) -> io::Error {
        match x {
            Error::IoError(x) => x,
            Error::NixError(x) => io::Error::new(io::ErrorKind::Other, format!("{:?}", x)),
            Error::ParseError(s) => io::Error::new(io::ErrorKind::Other, s),
        }
    }
}

impl From<num::ParseIntError> for Error {
    fn from(_: num::ParseIntError) -> Error {
        Error::ParseError(format!("failed to parse integer"))
    }
}

impl From<num::ParseFloatError> for Error {
    fn from(_: num::ParseFloatError) -> Error {
        Error::ParseError(format!("failed to parse float"))
    }
}
