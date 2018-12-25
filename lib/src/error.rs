use std::fmt::{Debug, Formatter, Result};
use std::{convert, option};

pub enum Error {
    CurlError(curl::Error),
    SerdeError(serde_json::Error),
    UnwrappingError,
    // ApiError(url, status_code, body)
    ApiError(String, u32, String),
    InvalidVersion,
    NoRecord
}

impl convert::From<curl::Error> for Error {
    fn from(e: curl::Error) -> Error {
        Error::CurlError(e)
    }
}

impl convert::From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::SerdeError(e)
    }
}

impl convert::From<option::NoneError> for Error {
    fn from(_: option::NoneError) -> Error {
        Error::UnwrappingError
    }
}


impl Debug for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Error::CurlError(e) => {
                write!(f, "HTTP Query Error({:?})", e)?;
            },
            Error::SerdeError(e) => {
                write!(f, "Parsing Error({:?})", e)?;
            },
            Error::UnwrappingError => {
                write!(f, "Err... Tried to unwrap some None there ;(")?;
            },
            Error::ApiError(url, status_code, body) => {
                let body = if body.len() > 150 {
                        format!("{}...OUTPUT TRUNCATED...{}", &body[0..100], &body[body.len()-50..body.len()])
                    } else {
                        body.clone()
                    };
                write!(f, "API Error(url = '{}', status_code = '{}', body = '{}')", url, status_code, &body)?;
            },
            Error::InvalidVersion => {
                write!(f, "Invalid Zone Version Requested")?;
            },
            Error::NoRecord => {
                write!(f, "Couldn't find a matching record")?;
            }
        }
        Ok(())
    }
}