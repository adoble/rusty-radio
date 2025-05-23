use heapless::{String, Vec};

use crate::error::ResponseError;

// Max size for a url
pub const MAX_URL_LEN: usize = 256;

/// This is limited for of a HTTP response that contains what is required for this project.
#[derive(Default, Clone)]
pub struct Response {
    pub code: Option<u16>,
    pub location: Option<String<MAX_URL_LEN>>,
    pub size: usize, //TODO
}

impl Response {
    // Function to handle header reading
    pub fn new(header_buffer: &[u8]) -> Result<Response, ResponseError> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut response = httparse::Response::new(&mut headers);

        let size = match response.parse(header_buffer)? {
            httparse::Status::Complete(size) => size,
            httparse::Status::Partial => {
                return Err(ResponseError::IncompleteHeaders);
            }
        };

        let headers = response.headers;
        let code = response.code;

        let redirect_location = headers
            .iter()
            .filter_map(|h| {
                if h.name.eq_ignore_ascii_case("location") {
                    Some(h.value)
                } else {
                    None
                }
            })
            .next();

        let redirect_url = if let Some(redirect_location) = redirect_location {
            let mut v = Vec::<u8, MAX_URL_LEN>::new();
            v.extend_from_slice(redirect_location)
                .map_err(|_| ResponseError::UrlParse)?;
            let s = String::from_utf8(v)?;
            Some(s)
        } else {
            None
        };

        Ok(Response {
            code,
            location: redirect_url,
            size,
        })
    }

    /// Returns true if a successful HTTP response  
    pub fn is_ok(&self) -> bool {
        let successful_range = 200..300;
        self.code
            .is_some_and(|code| successful_range.contains(&code))
    }

    /// Returns true if a  HTTP redirect  
    pub fn is_redirect(&self) -> bool {
        let redirect_range = 300..400;
        self.code.is_some_and(|code| redirect_range.contains(&code))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_response() {
        let header_buffer = include_bytes!("test_resources/example_response.txt");

        let r = Response::new(header_buffer);

        assert!(r.is_ok());

        let response = r.unwrap();

        assert_eq!(response.code.unwrap(), 200);
        assert!(response.location.is_some());
        assert_eq!(response.location.unwrap(), "http://redirect.com");
    }

    #[test]
    fn test_ok() {
        let mut response = Response {
            code: Some(200),
            ..Default::default()
        };

        assert!(response.is_ok());

        response.code = Some(226);

        assert!(response.is_ok());

        response.code = Some(300);

        assert!(!response.is_ok());

        response.code = None;

        assert!(!response.is_ok());
    }

    #[test]
    fn test_is_redirect() {
        let mut response = Response {
            code: Some(300),
            ..Default::default()
        };

        assert!(response.is_redirect());

        response.code = Some(301);

        assert!(response.is_redirect());

        response.code = Some(400);

        assert!(!response.is_redirect());

        response.code = None;

        assert!(!response.is_redirect());
    }
}
