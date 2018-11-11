pub static API_URL: &'static str = "https://api.online.net/api/v1";

#[derive(Debug)]
pub enum Error {
    CurlError(curl::Error),
    SerdeError(serde_json::Error),
    // ApiError(url, status_code, body)
    ApiError((String, u32, String)),
    InvalidVersion,
    NoRecord
}
