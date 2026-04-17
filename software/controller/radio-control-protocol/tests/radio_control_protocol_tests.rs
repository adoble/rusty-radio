use embedded_hal_mock::eh1::serial::{Mock as SerialMock, Transaction as SerialTransaction};

use radio_control_protocol::uart_handler::{Command, UartHandler, UartHandlerError};

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

    let mut uart_handler = UartHandler::new(&mut serial);

    let station_name = uart_handler
        .set_station(5)
        .expect("Error in setting station");

    assert_eq!("SWR3", station_name);

    serial.done();
}
