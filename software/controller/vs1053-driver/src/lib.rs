#![cfg_attr(not(test), no_std)]

//! This driver uses no chip select (xdcs or xcs) pins as this is managed by `SpiDevice`s.
//! If the hal only provides an `SpiBus` then see [this](https://github.com/rust-embedded/embedded-hal/blob/master/docs/migrating-from-0.2-to-1.0.md#for-end-users) on how to convert a `SpiBus`
//! into a `SpiDevice`
//!

// Note: The VS1053 uses a high speed and low speed SPI connnection.
// See https://docs.esp-rs.org/esp-hal/esp-hal/0.22.0/esp32c3/esp_hal/spi/master/index.html#shared-spi-access
// and  https://docs.embassy.dev/embassy-embedded-hal/git/default/shared_bus/asynch/spi/index.html
// for hints on how to set this up, but with the same SPI peripheral.

// Note (about clock speed from the VS1053 data sheet p23):
//  Although the timing is derived from the internal clock CLKI, the system always starts up in
// 1.0× mode, thus CLKI=XTALI. After you have configured a higher clock through SCI_CLOCKF
// and waited for DREQ to rise, you can use a higher SPI speed as well.

use embedded_hal_async::digital::Wait;
use embedded_hal_async::spi::{Operation, SpiDevice};

//use embedded_hal::digital::OutputPin;

mod registers;

//use embedded_hal_bus::spi::DeviceError;
use registers::Registers;

const SCI_READ: u8 = 0b0000_0011;
const SCI_WRITE: u8 = 0b0000_0010;

pub struct Vs1053Driver<SPI, DREQ> {
    spi_control_device: SPI,
    spi_data_device: SPI,

    dreq: DREQ,
}

impl<SPI, DREQ> Vs1053Driver<SPI, DREQ>
where
    SPI: SpiDevice,
    DREQ: Wait, // See https://docs.rs/embedded-hal-async/1.0.0/embedded_hal_async/digital/index.html
{
    pub fn new(
        spi_control_device: SPI,
        spi_data_device: SPI,
        dreq: DREQ,
    ) -> Result<Self, DriverError> {
        let driver = Vs1053Driver {
            spi_control_device,
            spi_data_device,
            dreq,
        };
        Ok(driver)
    }

    pub async fn sci_read(&mut self, addr: u8) -> Result<u16, DriverError> {
        // Note: XCS is managed by self.spi_device : SpiDevice

        self.dreq
            .wait_for_high()
            .await
            .map_err(|_| DriverError::DReq)?;

        let mut buf: [u8; 2] = [0; 2];

        self.spi_control_device
            .transaction(&mut [
                Operation::Write(&[SCI_READ, addr]),
                Operation::Read(&mut buf),
            ])
            .await
            .map_err(|_| DriverError::SpiRead)?;

        Ok(u16::from_be_bytes(buf))
    }

    #[allow(unused_variables)]
    pub async fn sci_write(&mut self, addr: u8, data: u16) -> Result<(), DriverError> {
        self.dreq
            .wait_for_high()
            .await
            .map_err(|_| DriverError::DReq)?;

        let mut buf: [u8; 4] = [0; 4];
        buf[0] = SCI_WRITE;
        buf[1] = addr;
        buf[2] = data.to_be_bytes()[0];
        buf[3] = data.to_be_bytes()[1];

        self.spi_control_device
            .transaction(&mut [Operation::Write(&buf)])
            .await
            .map_err(|_| DriverError::SpiWrite)?;

        Ok(())
    }

    // Destroys the driver and releases the peripherals
    pub fn release(self) -> (SPI, SPI, DREQ) {
        (self.spi_control_device, self.spi_data_device, self.dreq)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DriverError {
    SpiRead,
    SpiWrite,

    // An error in waiting for the DREQ signal
    DReq,
}

#[cfg(test)]
mod tests {
    use super::*;

    use embedded_hal_async::spi::SpiBus;
    //use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
    use embedded_hal_mock::eh1::digital::{
        Mock as PinMock, State as PinState, State, Transaction as PinTransaction,
    };
    use embedded_hal_mock::eh1::spi::{Mock as SpiMock, Transaction as SpiTransaction};

    #[async_std::test]
    //#[embassy_executor::test]
    async fn sci_read_test() {
        let spi_control_expectations = [
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![SCI_READ, 0x11]),
            SpiTransaction::read_vec(vec![0xAA, 0xBB]),
            SpiTransaction::transaction_end(),
        ];
        let spi_control_device = SpiMock::new(&spi_control_expectations);

        let spi_data_expectations: [SpiTransaction<u8>; 0] = [];
        let spi_data_device = SpiMock::new(&spi_data_expectations);

        // let mp3cs_expectations = [
        //     PinTransaction::set(PinState::Low),
        //     PinTransaction::set(PinState::High),
        // ];
        // let mp3cs_expectations: [PinTransaction; 0] = [];
        // let mp3cs = PinMock::new(&mp3cs_expectations);

        let dreq_expectations = [PinTransaction::wait_for_state(State::High)];
        //let dreq_expectations: [PinTransaction; 0] = [];
        let dreq = PinMock::new(&dreq_expectations);

        let mut driver = Vs1053Driver::new(spi_control_device, spi_data_device, dreq).unwrap();

        let value = driver.sci_read(0x11).await.unwrap();
        // 0xAABB = 43707
        assert_eq!(value, 43707);

        let (mut spi_control_device, mut spi_data_device, mut dreq) = driver.release();

        spi_control_device.done();
        spi_data_device.done();
        dreq.done();
    }

    #[async_std::test]
    async fn sci_write_test() {
        let spi_control_expectations = [
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![SCI_WRITE, 0x11, 0xAA, 0xBB]),
            SpiTransaction::transaction_end(),
        ];
        let spi_control_device = SpiMock::new(&spi_control_expectations);

        let spi_data_expectations: [SpiTransaction<u8>; 0] = [];
        let spi_data_device = SpiMock::new(&spi_data_expectations);

        // let mp3cs_expectations: [PinTransaction; 0] = [];
        // let mp3cs = PinMock::new(&mp3cs_expectations);

        //let dreq_expectations: [PinTransaction; 0] = [];
        let dreq_expectations = [PinTransaction::wait_for_state(State::High)];
        let dreq = PinMock::new(&dreq_expectations);

        let mut driver = Vs1053Driver::new(spi_control_device, spi_data_device, dreq).unwrap();

        // 0xAABB = 43707
        driver.sci_write(0x11, 43707).await.unwrap();

        let (mut spi_control_device, mut spi_data_device, mut dreq) = driver.release();

        spi_control_device.done();
        spi_data_device.done();
        dreq.done();
    }

    #[test]
    fn registers_conversion_test() {
        let val: u8 = Registers::Vs1053RegStatus as u8;
        assert_eq!(val, 0x01);
    }
}
