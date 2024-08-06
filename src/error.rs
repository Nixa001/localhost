use std::fmt;
use std::error::Error;
use std::io;

#[derive(Debug)]
pub enum ServerError {
    Io(io::Error),
    Http(u16),
    Internal(String),
    
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ServerError::Io(ref err) => write!(f, "IO error: {}", err),
            ServerError::Http(status) => write!(f, "HTTP error: {}", status),
            ServerError::Internal(ref err) => write!(f, "Internal error: {}", err),
        }
    }
}

impl Error for ServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            ServerError::Io(ref err) => Some(err),
            ServerError::Http(_) => None,
            ServerError::Internal(_) => None,
        }
    }
}

impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> ServerError {
        ServerError::Io(err)
    }
}

