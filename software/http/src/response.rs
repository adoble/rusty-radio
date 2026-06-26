use heapless::{String, Vec};

use crate::error::ResponseError;

// Max size for a url
pub const MAX_URL_LEN: usize = 256;

/// This is limited for of a HTTP response that contains what is required for this project.
#[derive(Default, Clone)]
pub struct Response {
    pub status_code: ResponseStatusCode,
    pub location: Option<String<MAX_URL_LEN>>,
    pub size: usize, //TODO
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum ResponseStatusCode {
    Informational(u16),
    Successful(u16),
    Redirection(u16),
    ClientError(u16),
    ServerError(u16),
    Invalid(u16),
    #[default]
    Unknown,
}

impl From<u16> for ResponseStatusCode {
    fn from(value: u16) -> Self {
        match value {
            100..200 => Self::Informational(value),
            200..300 => Self::Successful(value),
            300..400 => Self::Redirection(value),
            400..500 => Self::ClientError(value),
            500..600 => Self::ServerError(value),
            _ => Self::Invalid(value),
        }
    }
}

impl From<Option<u16>> for ResponseStatusCode {
    fn from(value: Option<u16>) -> Self {
        match value {
            Some(status_code) => ResponseStatusCode::from(status_code),
            None => ResponseStatusCode::Unknown,
        }
    }
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
        let code = ResponseStatusCode::from(response.code);

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
            status_code: code,
            location: redirect_url,
            size,
        })
    }

    pub fn status_code(&self) -> ResponseStatusCode {
        self.status_code.clone()
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

        assert_eq!(response.status_code, ResponseStatusCode::Successful(200));
        assert!(response.location.is_some());
        assert_eq!(response.location.unwrap(), "http://redirect.com");
    }

    #[test]
    fn test_response_status_code_from_u16() {
        let response_status_code = ResponseStatusCode::from(200);
        assert_eq!(response_status_code, ResponseStatusCode::Successful(200));

        let response_status_code = ResponseStatusCode::from(300);
        assert_eq!(response_status_code, ResponseStatusCode::Redirection(300));

        let response_status_code = ResponseStatusCode::from(306);
        assert_eq!(response_status_code, ResponseStatusCode::Redirection(306));

        let response_status_code = ResponseStatusCode::from(499);
        assert_eq!(response_status_code, ResponseStatusCode::ClientError(499));

        let response_status_code = ResponseStatusCode::from(500);
        assert_eq!(response_status_code, ResponseStatusCode::ServerError(500));

        let response_status_code = ResponseStatusCode::from(600);
        assert_eq!(response_status_code, ResponseStatusCode::Invalid(600));
    }

    #[test]
    fn test_response_status_code_from_option_u16() {
        let response_status_code = ResponseStatusCode::from(Some(200));
        assert_eq!(response_status_code, ResponseStatusCode::Successful(200));

        let response_status_code = ResponseStatusCode::from(Some(300));
        assert_eq!(response_status_code, ResponseStatusCode::Redirection(300));

        let response_status_code = ResponseStatusCode::from(Some(306));
        assert_eq!(response_status_code, ResponseStatusCode::Redirection(306));

        let response_status_code = ResponseStatusCode::from(Some(499));
        assert_eq!(response_status_code, ResponseStatusCode::ClientError(499));

        let response_status_code = ResponseStatusCode::from(Some(500));
        assert_eq!(response_status_code, ResponseStatusCode::ServerError(500));

        let response_status_code = ResponseStatusCode::from(Some(600));
        assert_eq!(response_status_code, ResponseStatusCode::Invalid(600));

        let response_status_code = ResponseStatusCode::from(None);
        assert_eq!(response_status_code, ResponseStatusCode::Unknown);
    }
}
