#![cfg_attr(not(test), no_std)]

// TODO Change this into a fluent API.

mod error;
mod request;
mod response;

pub use error::RequestError;
pub use request::Method;
pub use request::Request;
pub use response::{Response, MAX_URL_LEN};
