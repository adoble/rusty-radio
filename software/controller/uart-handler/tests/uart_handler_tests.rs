/// Note that we're using the non-blocking serial traits
use embedded_hal_mock::eh1::serial::{Mock as SerialMock, Transaction as SerialTransaction};
use embedded_hal_nb::serial::{Read, Write};

use uart_handler::send_hello;

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
