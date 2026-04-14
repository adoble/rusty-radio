#[derive(Debug)]
//#[cfg_attr(not(test), derive(defmt::Format))] // Only used when running on target hardware
pub enum UartHandlerError {
    NotSupportedForDeviceSource,
    ReadingQueryResponse,
    ParseResponse,
    NonUTF8,
    SendCommand,
    SourceNotKnown,
    BooleanParse,
    OutOfRange,
    InvalidString,
    IllFormedReponse,
    CannotConvert,
    Timeout,
    Read,
    Write,
    Unimplemented,
}
