#![no_std]

extern crate alloc;

use heapless::String;

use crate::error::HttpBuilderError;

use crate::request::{Method, Request};
pub enum RequestBuilder {
    Ok(Request),
    Err(HttpBuilderError),
}

impl RequestBuilder {
    /// Creates a new `RequestBuilder` with the specified HTTP method and path.
    pub fn new(method: Method, path: &str) -> Self {
        if let Ok(request) = Request::new(method, path) {
            return RequestBuilder::Ok(request);
        } else {
            return RequestBuilder::Err(HttpBuilderError::StringConversionError);
        }
    }

    /// Adds a header to the request.
    pub fn header(mut self, key: &str, value: &str) -> Self {
        if let Ok(ref mut request) = self {
            request
                .header(key, value)
                .map_err(|err| RequestBuilder::Err(err))
        } else {
            RequestBuilder::Err(HttpBuilderError::StringConversionError)
        }
    }

    /// Adds a host
    pub fn host(mut self, host: &str) -> Self {
        match self {
            RequestBuilder::Ok(ref mut request) => {
                if let Err(e) = request.host("Host", host) {
                    return RequestBuilder::Err(e);
                }
            }
            RequestBuilder::Err(err) => RequestBuilder::Err(err),
        }
        self
    }

    /// Sets the body of the request.
    pub fn body(mut self, body: &str) -> Self {
        match self {
            RequestBuilder::Ok(ref mut request) => {
                if let Err(e) = request.body(body) {
                    return RequestBuilder::Err(e);
                }
            }
            RequestBuilder::Err(err) => RequestBuilder::Err(err),
        }
        self
    }

    /// Builds the HTTP request as a string.
    pub fn build(self) -> Result<String<REQUEST_SIZE>, HttpBuilderError> {
        match self {
            RequestBuilder::Ok(request) => {
                let request_str = request.build();
                Ok(request_str)
            }
            RequestBuilder::Err(err) => Err(err),
        }

        self
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
