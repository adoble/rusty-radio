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
