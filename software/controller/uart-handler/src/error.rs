#[derive(Debug)]
//#[cfg_attr(not(test), derive(defmt::Format))] // Only used when running on target hardware
pub enum UartHandlerError {
    SerialWrite(embedded_hal_nb::serial::ErrorKind),
    SerialRead(embedded_hal_nb::serial::ErrorKind),
    ResponseTooLarge,
    NonUTF8,
    IllFormedReponse,
    ParameterTooLarge,

    ClientCannotHandleCommand,
    ClientSentUnknownErrorCode,
}
