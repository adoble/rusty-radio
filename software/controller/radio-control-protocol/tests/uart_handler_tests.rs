/// Note that we're using the non-blocking serial traits
use embedded_hal_mock::eh1::serial::{Mock as SerialMock, Transaction as SerialTransaction};

use radio_control_protocol::uart_handler::{Command, UartHandler, UartHandlerError};

use heapless::{String, Vec};

#[test]
fn test_send_command() {
    // Configure expectations
    let tx_message = "STA:5;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler: UartHandler<'_, _, 40, 5> = UartHandler::new(&mut serial);

    let mut parameters = Vec::<&str, 5>::new();
    parameters.push("5").unwrap();

    let r = uart_handler.send_command(Command::Station, parameters);

    assert!(r.is_ok());

    serial.done();
}

#[test]
fn test_receive_response_with_single_parameter() {
    let rx_message = "ACK:SWR3;";
    let expectations = [SerialTransaction::read_many(rx_message.as_bytes())];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters);

    assert!(r.is_ok());

    assert_eq!(1, parameters.len());
    assert_eq!("SWR3", parameters[0].as_str());

    serial.done();
}

#[test]
fn test_receive_response_with_many_parameters() {
    let rx_message = "ACK:value1,value2,value3;";
    let expectations = [SerialTransaction::read_many(rx_message.as_bytes())];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters);

    assert!(r.is_ok());

    assert_eq!(3, parameters.len());
    assert_eq!("value1", parameters[0].as_str());
    assert_eq!("value2", parameters[1].as_str());
    assert_eq!("value3", parameters[2].as_str());

    serial.done();
}

#[test]
fn test_receive_response_with_no_parameters() {
    let rx_message = "ACK:;";
    let expectations = [SerialTransaction::read_many(rx_message.as_bytes())];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters);

    assert!(r.is_ok());

    assert_eq!(0, parameters.len());

    serial.done();
}

#[test]
fn test_receive_response_with_error() {
    let rx_message = "ERR:001;";
    let expectations = [SerialTransaction::read_many(rx_message.as_bytes())];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters);

    assert!(r.is_err());

    if let Err(error_code) = r {
        assert_eq!(UartHandlerError::ClientCannotHandleCommand, error_code)
    };

    serial.done();
}

#[test]
fn test_receive_response_with_unknown_error() {
    let rx_message = "ERR:999;";
    let expectations = [SerialTransaction::read_many(rx_message.as_bytes())];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters);

    assert!(r.is_err());

    if let Err(error_code) = r {
        assert_eq!(UartHandlerError::ClientSentUnknownErrorCode, error_code)
    };

    serial.done();
}

#[test]
fn test_receive_ill_formed_response() {
    let rx_message = "ACK;";
    let expectations = [SerialTransaction::read_many(rx_message.as_bytes())];

    let mut serial = SerialMock::new(&expectations);

    let mut uart_handler = UartHandler::new(&mut serial);

    let mut parameters = Vec::<String<40>, 5>::new();

    let r = uart_handler.receive_response(&mut parameters);

    assert!(r.is_err());

    if let Err(error_code) = r {
        assert_eq!(UartHandlerError::IllFormedReponse, error_code)
    };

    serial.done();
}

// #[test]
// fn test_set_station() {
//     // Configure expectations
//     let tx_message = "STA:5;";
//     let rx_message = "ACK:SWR3;";
//     let expectations = [
//         SerialTransaction::write_many(tx_message.as_bytes()),
//         SerialTransaction::flush(),
//         SerialTransaction::read_many(rx_message.as_bytes()),
//     ];

//     let mut serial = SerialMock::new(&expectations);

//     let mut uart_handler = UartHandler::new(&mut serial);

//     let station_name = uart_handler
//         .set_station(5)
//         .expect("Error in setting station");

//     assert_eq!("SWR3", station_name);

//     serial.done();
// }

#[test]
fn test_command_conversion_to_str() {
    let cmd = Command::Station;

    let cmd_str: String<3> = cmd.stringify();

    // let mut s = String::<3>::new();
    // s.push_str("STA");
    assert_eq!("STA", cmd_str);
}
