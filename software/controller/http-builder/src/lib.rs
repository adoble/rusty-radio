#![no_std]

/// Example usage:
/// ```no-compile
/// use http_builder::{RequestBuilder, Method};
/// use heapless::String;
///
/// use crate::error::HttpBuilderError;
///     
/// fn make_request() ->  Result<(), HttpBuilderError> {
///    let request = RequestBuilder::new(Method::GET, "/path/to/resource")?
///        .host("Host", "example.com")?
///        .header("User-Agent", "MyClient/1.0")?
///        .body("Hello, world!")?
///       .build();
///                                 
///
///     assert_eq!(
///        request,
///       "GET /path/to/resource HTTP/1.1\r\nHost: example.com\r\nUser-Agent: MyClient/1.0\r\n\r\nHello, world!"
///      );
///
///     Ok(())
/// }
/// ```
extern crate alloc;

use heapless::String;

use crate::error::HttpBuilderError;

mod error;

/// The maximum size of the path in the request.
const PATH_SIZE: usize = 128;

/// The maximum size of the headers in the request.
const HEADER_SIZE: usize = 512;

const BODY_SIZE: usize = 1024;
//// The total size of the request string
const REQUEST_SIZE: usize = PATH_SIZE + HEADER_SIZE + BODY_SIZE + 64; // 64 for HTTP version and CRLF

/// Represents an HTTP method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

impl Method {
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::PATCH => "PATCH",
            Method::HEAD => "HEAD",
            Method::OPTIONS => "OPTIONS",
        }
    }
}

/// A lightweight HTTP request builder for no-std environments.
pub struct RequestBuilder {
    method: Method,
    path: String<PATH_SIZE>,
    headers: String<HEADER_SIZE>,
    body: Option<String<BODY_SIZE>>,
}

impl RequestBuilder {
    /// Creates a new `RequestBuilder` with the specified HTTP method and path.
    pub fn new(method: Method, path: &str) -> Result<Self, HttpBuilderError> {
        Ok(Self {
            method,
            path: String::try_from(path).map_err(|_| HttpBuilderError::StringConversionError)?,
            headers: String::new(),
            body: None,
        })
    }

    /// Adds a header to the request.
    pub fn header(mut self, key: &str, value: &str) -> Result<Self, HttpBuilderError> {
        // TOD remove the let _ and add proper error handling
        self.headers
            .push_str(key)
            .map_err(|_| HttpBuilderError::StringPushError)?;
        self.headers
            .push_str(": ")
            .map_err(|_| HttpBuilderError::StringPushError)?;
        self.headers
            .push_str(value)
            .map_err(|_| HttpBuilderError::StringPushError)?;
        self.headers
            .push_str("\r\n")
            .map_err(|_| HttpBuilderError::StringPushError)?;
        Ok(self)
    }

    /// Adds a host
    pub fn host(mut self, host: &str) -> Result<Self, HttpBuilderError> {
        self.header("Host", host)
    }

    /// Sets the body of the request.
    pub fn body(mut self, body: &str) -> Result<Self, HttpBuilderError> {
        self.body =
            Some(String::try_from(body).map_err(|_| HttpBuilderError::StringConversionError)?);
        Ok(self)
    }

    /// Builds the HTTP request as a string.
    pub fn build(self) -> String<REQUEST_SIZE> {
        let mut request = String::new();
        let _ = request.push_str(self.method.as_str());
        let _ = request.push_str(" ");
        let _ = request.push_str(&self.path);
        let _ = request.push_str(" HTTP/1.1\r\n");
        let _ = request.push_str(&self.headers);
        let _ = request.push_str("\r\n");
        if let Some(body) = self.body {
            let _ = request.push_str(&body);
        }
        request
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_get() -> Result<(), HttpBuilderError> {
        let request = RequestBuilder::new(Method::GET, "/pub/WWW/")?
            .host("www.example.org")?
            .build();

        assert_eq!(
            request,
            "GET /pub/WWW/ HTTP/1.1\r\nHost: www.example.org\r\n\r\n"
        );

        Ok(())
    }

    #[test]
    fn test_request_builder() -> Result<(), HttpBuilderError> {
        let request = RequestBuilder::new(Method::GET, "/path/to/resource")?
            .host("example.com")?
            .header("User-Agent", "MyClient/1.0")?
            .body("Hello, world!")?
            .build();

        assert_eq!(
            request,
            "GET /path/to/resource HTTP/1.1\r\nHost: example.com\r\nUser-Agent: MyClient/1.0\r\n\r\nHello, world!"
        );
        Ok(())
    }
}
