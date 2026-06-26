#![cfg_attr(not(test), no_std)]

//! # Minimal HTTP Library for Embedded Rust
//!
//! This crate provides a lightweight, no_std-compatible HTTP implementation suitable for
//! embedded and resource-constrained environments.
//!
//! ## Features
//! - HTTP request and response parsing
//! - Support for common HTTP methods
//! - Minimal memory usage, designed for microcontrollers
//! - Simple error handling
//!
//! ## Modules
//! - `request`: HTTP request construction and parsing
//! - `response`: HTTP response parsing and status code handling
//! - `error`: Error types for request and response operations
//!

mod error;
mod request;
mod response;

pub use error::{RequestError, ResponseError};
pub use request::Method;
pub use request::Request;
pub use response::{Response, ResponseStatusCode, MAX_URL_LEN};
