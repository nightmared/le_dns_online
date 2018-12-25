#![feature(try_trait)]

mod api;
pub use crate::api::*;
pub mod net;
pub mod error;
#[cfg(test)]
mod test;

pub static API_URL: &'static str = "https://api.online.net/api/v1";