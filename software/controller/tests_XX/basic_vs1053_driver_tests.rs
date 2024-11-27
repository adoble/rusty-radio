use super::*;

use super::vs1053_driver::{DriverError, Vs1053Driver};

use embedded_hal_mock::eh1::spi::{Mock as SpiMock, Transaction as SpiTransaction};
use embedded_hal_mock::en1::digital::{
    Mock as PinMock, State as PinState, Transaction as PinTransaction,
};

//#[async_std::test]
#[embassy_executor::test]
async fn sci_read_test() {
    let expectations = [
        SpiTransaction::write_vec(vec![READ, 0x11]),
        SpiTransaction::read_vec[vec![0xAA, 0xBB]],
    ];

    let xdcs_expectations = [PinTransaction::set(PinState::Low)];
    let mut xdcs = PinMock::new(xdcs_expectations);
    let mp3cs_expectations = [
        PinTransaction::set(PinState::Low),
        PinTransaction::set(PinState::High),
    ];
    let mut mp3cs = PinMock::new(xdcs_expectations);

    let dreq_expectations = [
        PinTransaction::get(PinState::High),
        PinTransaction::get(PinState::Low),
    ];
    let mut dreq = PinMock::new(dreq_expectations);

    let spi = SpiMock(&expectations);
    let driver = Vs1053Driver::new(spi, mp3cs, xdcs, dreq).unwrap();

    let mut buf: [u8; 2] = [0; 2];
    let x = driver.sci_read(0x11, &buf).await.unwrap();
}
