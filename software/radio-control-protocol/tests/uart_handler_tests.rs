use std::str::FromStr;

use embedded_io_async_mock::{Mock as SerialMock, Transaction as SerialTransaction};

use radio_control_protocol::{
    radio_control_protocol::MAX_PARAMETER_LEN,
    uart_handler::{Command, UartHandler, UartHandlerError},
};

use heapless::{String, Vec};

#[test]
fn test_command_conversion_to_bytes() {
    let cmd = Command::Station;

    let bytes: [u8; 3] = (&cmd).into();

    assert_eq!([b'S', b'T', b'A'], bytes);
}

#[test]
fn test_command_conversion_from_bytes() {
    let cmd = Command::try_from(b"STA").expect("Command conversion failed!");
    assert_eq!(cmd, Command::Station);
}

#[test]
fn test_command_conversion_from_bytes_with_error() {
    assert_eq!(
        Err(UartHandlerError::UnknownCommand),
        Command::try_from(b"XXX")
    );
}

#[tokio::test]
async fn test_send_command() {
    // Configure expectations
    let tx_message = b"STA:5;";
    let expectations = [
        SerialTransaction::write_many(tx_message),
        SerialTransaction::flush(),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler: UartHandler<'_, _, 40, 5> = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<MAX_PARAMETER_LEN>, 5>::new();
    let parameter = String::<MAX_PARAMETER_LEN>::from_str("5").unwrap();
    parameters.push(parameter).unwrap();

    let r = uart_handler
        .send_command(Command::Station, parameters)
        .await;

    assert!(r.is_ok());

    serial.done();
}

#[tokio::test]
async fn test_send_command_with_no_parameters() {
    // Configure expectations
    let tx_message = b"STA:;";
    let expectations = [
        SerialTransaction::write_many(tx_message),
        SerialTransaction::flush(),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler: UartHandler<'_, _, 40, 5> = UartHandler::new(&mut serial);

    let parameters = Vec::<String<MAX_PARAMETER_LEN>, 5>::new();

    let r = uart_handler
        .send_command(Command::Station, parameters)
        .await;

    assert!(r.is_ok());

    serial.done();
}

#[tokio::test]
async fn test_receive_response_with_single_parameter() {
    let rx_message = b"ACK:SWR3;";
    let expectations = [SerialTransaction::read_many(rx_message)];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters).await;

    assert!(r.is_ok());

    assert_eq!(1, parameters.len());
    assert_eq!("SWR3", parameters[0].as_str());

    serial.done();
}

#[tokio::test]
async fn test_receive_response_with_many_parameters() {
    let rx_message = b"ACK:value1,value2,value3;";
    let expectations = [SerialTransaction::read_many(rx_message)];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters).await;

    assert!(r.is_ok());

    assert_eq!(3, parameters.len());
    assert_eq!("value1", parameters[0].as_str());
    assert_eq!("value2", parameters[1].as_str());
    assert_eq!("value3", parameters[2].as_str());

    serial.done();
}

#[tokio::test]
async fn test_receive_response_with_no_parameters() {
    let rx_message = b"ACK:;";
    let expectations = [SerialTransaction::read_many(rx_message)];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters).await;

    assert!(r.is_ok());

    assert_eq!(0, parameters.len());

    serial.done();
}

#[tokio::test]
async fn test_receive_response_with_error() {
    let rx_message = b"ERR:001;";
    let expectations = [SerialTransaction::read_many(rx_message)];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters).await;

    assert!(r.is_err());

    if let Err(error_code) = r {
        assert_eq!(UartHandlerError::ClientCannotHandleCommand, error_code)
    };

    serial.done();
}

#[tokio::test]
async fn test_receive_response_with_unknown_error() {
    let rx_message = b"ERR:999;";
    let expectations = [SerialTransaction::read_many(rx_message)];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters).await;

    assert!(r.is_err());

    if let Err(error_code) = r {
        assert_eq!(UartHandlerError::ClientSentUnknownErrorCode, error_code)
    };

    serial.done();
}

#[tokio::test]
async fn test_receive_ill_formed_response() {
    let rx_message = b"ACK;";
    let expectations = [SerialTransaction::read_many(rx_message)];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters).await;

    assert!(r.is_err());

    if let Err(error_code) = r {
        assert_eq!(UartHandlerError::IllFormedReponse, error_code)
    };

    serial.done();
}

#[tokio::test]
async fn test_send_empty_acknowledge() {
    // Configure expectations
    let tx_message = b"ACK:;";
    let expectations = [
        SerialTransaction::write_many(tx_message),
        SerialTransaction::flush(),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler: UartHandler<'_, _, 40, 5> = UartHandler::new(&mut serial);

    let parameters = Vec::<String<MAX_PARAMETER_LEN>, 5>::new();
    //let parameter = String::<MAX_PARAMETER_LEN>::from_str("5").unwrap();
    //parameters.push(parameter).unwrap();

    let r = uart_handler.send_acknowledge(&parameters).await;

    assert!(r.is_ok());

    serial.done();
}

#[tokio::test]
async fn test_send_acknowledge() {
    // Configure expectations
    let tx_message = b"ACK:SWR3,RPR2;";
    let expectations = [
        SerialTransaction::write_many(tx_message),
        SerialTransaction::flush(),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler: UartHandler<'_, _, 40, 5> = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<MAX_PARAMETER_LEN>, 5>::new();

    let parameter = String::<MAX_PARAMETER_LEN>::from_str("SWR3").unwrap();
    parameters.push(parameter).unwrap();
    let parameter = String::<MAX_PARAMETER_LEN>::from_str("RPR2").unwrap();
    parameters.push(parameter).unwrap();

    let r = uart_handler.send_acknowledge(&parameters).await;

    assert!(r.is_ok());

    assert_eq!(parameters.len(), 2);

    assert_eq!(parameters.to_vec(), ["SWR3", "RPR2"]);

    serial.done();
}

#[tokio::test]
async fn test_send_error() {
    // Configure expectations
    let tx_message = b"ERR:101;";
    let expectations = [
        SerialTransaction::write_many(tx_message),
        SerialTransaction::flush(),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler: UartHandler<'_, _, 40, 5> = UartHandler::new(&mut serial);

    let r = uart_handler.send_error(101).await;

    assert!(r.is_ok());

    serial.done();
}

#[tokio::test]
async fn test_read_frame() {
    let rx_message = b"CFG:7,1,2,3,4;";
    let expectations = [SerialTransaction::read_many(rx_message)];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler: UartHandler<'_, _, 40, 5> = UartHandler::new(&mut serial);

    let r = uart_handler.receive_frame().await;

    assert_eq!(r, Ok(rx_message.len()));

    serial.done();
}

#[tokio::test]
async fn test_read_frame_with_buffer_overflow() {
    const FRAME_BUFFER_LEN: usize = 80;
    let rx_message = b"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX \
    XXXXXXXXXXXXXXXXXXXXXX;";

    let expectations = [SerialTransaction::read_many(rx_message)];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler: UartHandler<'_, _, 40, 5> = UartHandler::new(&mut serial);

    let r = uart_handler.receive_frame().await;
    assert_eq!(r, Err(UartHandlerError::FrameBufferOverflow));

    let r = uart_handler.receive_frame().await;
    assert_eq!(r, Ok(2));

    serial.done();
}
