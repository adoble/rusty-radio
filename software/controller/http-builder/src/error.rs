#[derive(Debug)]
pub enum HttpBuilderError {
    /// Error when trying to convert a string to a fixed-size string.
    StringConversionError,
    /// Error when trying to push a string into a fixed-size string.
    StringPushError,
    /// Error when trying to create a new `RequestBuilder`.
    RequestBuilderCreationError,
}
