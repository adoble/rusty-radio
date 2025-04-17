#![no_std]

// TODO Change this into a fluent API.

mod error;
mod request;

pub use error::HttpBuilderError;
pub use request::Method;
pub use request::Request;
