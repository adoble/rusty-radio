#[derive(Debug)]
pub enum RequestError {
    /// Error when trying to convert a string to a fixed-size string.
    StringConversionError,
    /// Error when trying to push a string into a fixed-size string.
    StringPushError,
    /// Error when trying to create a new `RequestBuilder`.
    RequestBuilderCreationError,
}

#[derive(Debug)]
pub enum ResponseError {
    /// Unexpected EOF
    UnexpectedEof,

    /// Failed to parse headers
    HeaderParse(httparse::Error),

    /// Failed to parse a URL
    //UrlParse(nourl::Error),
    UrlParse,

    /// Failed to convert header to string
    StringConversion(core::str::Utf8Error),

    /// Headers incomplete
    IncompleteHeaders,

    /// Buffer overflow
    BufferOverflow,
}

impl From<httparse::Error> for ResponseError {
    fn from(e: httparse::Error) -> ResponseError {
        ResponseError::HeaderParse(e)
    }
}

impl From<nourl::Error> for ResponseError {
    fn from(_e: nourl::Error) -> ResponseError {
        ResponseError::UrlParse
    }
}

impl From<core::str::Utf8Error> for ResponseError {
    fn from(e: core::str::Utf8Error) -> ResponseError {
        ResponseError::StringConversion(e)
    }
}
