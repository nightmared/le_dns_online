use std::fmt::{Debug, Formatter, Result};

pub static API_URL: &'static str = "https://api.online.net/api/v1";

pub enum Error {
    CurlError(curl::Error),
    SerdeError(serde_json::Error),
    // ApiError(url, status_code, body)
    ApiError(String, u32, String),
    InvalidVersion,
    NoRecord
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