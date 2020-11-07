mod api;
pub use crate::api::*;
pub mod net;
pub mod error;
mod bind;

pub static API_URL: &'static str = "https://api.online.net/api/v1";
