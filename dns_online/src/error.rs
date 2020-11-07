use std::convert;
use std::fmt::{Debug, Formatter, Result};

/// Returned when a request can't be completed and isn't expected, this may allow you to determine
/// why this "exception" was thrown
pub struct APIError {
    pub url: String,
    pub body: Vec<u8>,
    pub status_code: u32,
}

/// The various errors thay may be returned by the API, ranging from network errors to unproper use
/// of the API, and through serializations errors.
pub enum Error {
    /// Network error or invalid query
    CurlError(curl::Error),
    /// JSON parsing error
    SerdeError(serde_json::Error),
    /// The remote endpoint returned a HTTP error code
    ApiError(APIError),
    /// A conversion to an utf-8 string failed
    FromUtf8Error(std::string::FromUtf8Error),
    /// The active zone cannot be modified,
    ActiveZoneForbidden,
    /// The zone specified is invalid or nonexistent
    InvalidVersion,
    /// Occurs when a POST is made without any argument
    InvalidPost,
    /// No matching record found
    NoRecord,
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

impl convert::From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Error {
        Error::FromUtf8Error(e)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Error::CurlError(e) => {
                write!(f, "HTTP Query Error({:?})", e)?;
            }
            Error::SerdeError(e) => {
                write!(f, "Parsing Error({:?})", e)?;
            }
            Error::FromUtf8Error(e) => {
                write!(f, "UTF8 Conversion Error({:?})", e)?;
            }
            Error::ApiError(e) => {
                let body_str = String::from_utf8(e.body.clone()).unwrap();
                let body = if body_str.len() > 150 {
                    format!(
                        "{}...OUTPUT TRUNCATED...{}",
                        &body_str[0..100],
                        &body_str[body_str.len() - 50..body_str.len()]
                    )
                } else {
                    format!("{}", body_str)
                };
                write!(
                    f,
                    "API Error(url = '{}', status_code = '{}', body = '{}')",
                    e.url, e.status_code, &body
                )?;
            }
            Error::ActiveZoneForbidden => {
                write!(f, "Mutation of the currently active Zone is Forbidden")?;
            }
            Error::InvalidVersion => {
                write!(f, "Invalid Zone Version Requested")?;
            }
            Error::InvalidPost => {
                write!(f, "You tried to submit a POST with no argument")?;
            }
            Error::NoRecord => {
                write!(f, "Couldn't find a matching record")?;
            }
        }
        Ok(())
    }
}
