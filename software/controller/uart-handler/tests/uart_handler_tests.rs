/// Note that we're using the non-blocking serial traits
use embedded_hal_mock::eh1::serial::{Mock as SerialMock, Transaction as SerialTransaction};
use embedded_hal_nb::serial::{Read, Write};

use uart_handler::{send_hello, set_station};

#[test]
fn test_serial() {
    // Configure expectations
    let expectations = [
        SerialTransaction::write_many("Hello".as_bytes()),
        SerialTransaction::flush(),
    ];

    let mut serial = SerialMock::new(&expectations);

    let _ = send_hello(&mut serial);

    // When you believe there are no more calls on the mock,
    // call done() to assert there are no pending transactions.
    serial.done();
}

#[test]
fn test_set_station() {
    // Configure expectations
    let tx_message = "STA:5;";
    let rx_message = "ACK:;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let _r = set_station(&mut serial, 5);

    serial.done();
}
