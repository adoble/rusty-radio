use embedded_hal_mock::eh1::serial::{Mock as SerialMock, Transaction as SerialTransaction};

use radio_control_protocol::{
    RadioControlProtocol, radio_control_protocol::RadioControlProtocolError,
    uart_handler::UartHandlerError,
};

#[test]
fn test_set_station() {
    // Configure expectations
    let tx_message = "STA:5;";
    let rx_message = "ACK:SWR3;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let station_name = radio_control_protocol
        .set_station(5)
        .expect("Error in setting station");

    assert_eq!("SWR3", station_name);

    serial.done();
}

#[test]
fn test_set_station_with_error() {
    // Configure expectations
    let tx_message = "STA:5;";
    let rx_message = "ERR:001;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let result = radio_control_protocol.set_station(5);

    assert_eq!(
        Err(RadioControlProtocolError::Uart(
            UartHandlerError::ClientCannotHandleCommand
        )),
        result
    );

    serial.done();
}

#[test]
fn test_set_preset() {
    // Configure expectations
    let tx_message = "PRE:2;";
    let rx_message = "ACK:RPR1;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let station_name = radio_control_protocol
        .set_preset(2)
        .expect("Error in setting preset");

    assert_eq!("RPR1", station_name);

    serial.done();
}

#[test]
fn test_query_config() {
    // Configure expectations
    let tx_message = "CFG:;";
    let rx_message = "ACK:42;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let number_stations = radio_control_protocol
        .query_config()
        .expect("Error in setting config");

    assert_eq!(number_stations, 42);

    serial.done();
}

#[test]
fn test_query_config_with_parameter_error() {
    // Configure expectations
    let tx_message = "CFG:;";
    let rx_message = "ACK:;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let r = radio_control_protocol.query_config();

    assert_eq!(
        r,
        Err(RadioControlProtocolError::IncorrectNumberParametersReturned)
    );

    serial.done();
}

#[test]
fn test_query_config_with_parse_error() {
    // Configure expectations
    let tx_message = "CFG:;";
    let rx_message = "ACK:XXX;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let r = radio_control_protocol.query_config();

    assert_eq!(r, Err(RadioControlProtocolError::ParseParameter));

    serial.done();
}
