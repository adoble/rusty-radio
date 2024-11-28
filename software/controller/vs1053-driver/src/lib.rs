#![cfg_attr(not(test), no_std)]

//! This driver uses no chip select (xdcs) pin as this is managed by `SpiDevice`.
//! If the hal only provides an `SpiBus` then see [this](https://github.com/rust-embedded/embedded-hal/blob/master/docs/migrating-from-0.2-to-1.0.md#for-end-users) on how to convert a `SpiBus`
//! into a `SpiDevice`

use embedded_hal_async::digital::Wait;
use embedded_hal_async::spi::{Operation, SpiDevice};

use embedded_hal::digital::OutputPin;

const READ: u8 = 0b0000_0011;

pub struct Vs1053Driver<SPI, MP3CS, DREQ> {
    spi_device: SPI,
    mp3cs: MP3CS,
    dreq: DREQ,
}

impl<SPI, MP3CS, DREQ> Vs1053Driver<SPI, MP3CS, DREQ>
where
    SPI: SpiDevice,
    MP3CS: OutputPin,
    DREQ: Wait, // See https://docs.rs/embedded-hal-async/1.0.0/embedded_hal_async/digital/index.html
{
    pub fn new(spi_device: SPI, mp3cs: MP3CS, dreq: DREQ) -> Result<Self, DriverError> {
        let driver = Vs1053Driver {
            spi_device,
            mp3cs,
            dreq,
        };
        Ok(driver)
    }

    pub async fn sci_read(&mut self, addr: u8) -> Result<u16, DriverError> {
        // Note: XDCS is managed by self.spi_device : SpiDevice

        let mut buf: [u8; 2] = [0; 2];

        self.spi_device
            .transaction(&mut [Operation::Write(&[READ, addr]), Operation::Read(&mut buf)])
            .await
            .map_err(|_| DriverError::SpiRead)?;

        Ok(u16::from_be_bytes(buf))
    }

    #[allow(unused_variables)]
    pub async fn sci_write(&mut self, _addr: u8, _data: u16) -> Result<(), DriverError> {
        todo!()
    }

    // Destroys the driver and releases the peripherals
    pub fn release(self) -> (SPI, MP3CS, DREQ) {
        (self.spi_device, self.mp3cs, self.dreq)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DriverError {
    SpiRead,
    SpiWrite,
}

#[cfg(test)]
mod tests {
    use super::*;

    use embedded_hal_async::spi::SpiBus;
    //use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
    use embedded_hal_mock::eh1::digital::{
        Mock as PinMock, State as PinState, Transaction as PinTransaction,
    };
    use embedded_hal_mock::eh1::spi::{Mock as SpiMock, Transaction as SpiTransaction};

    #[async_std::test]
    //#[embassy_executor::test]
    async fn sci_read_test() {
        // See https://github.com/rust-embedded/embedded-hal/blob/master/docs/migrating-from-0.2-to-1.0.md#for-end-users

        let spi_expectations = [
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![READ, 0x11]),
            SpiTransaction::read_vec(vec![0xAA, 0xBB]),
            SpiTransaction::transaction_end(),
        ];
        let spi_device = SpiMock::new(&spi_expectations); // Assuming this is a SpiBus

        // let mp3cs_expectations = [
        //     PinTransaction::set(PinState::Low),
        //     PinTransaction::set(PinState::High),
        // ];
        let mp3cs_expectations: [PinTransaction; 0] = [];
        let mp3cs = PinMock::new(&mp3cs_expectations);

        // let dreq_expectations = [
        //     PinTransaction::get(PinState::High),
        //     PinTransaction::get(PinState::Low),
        // ];
        let dreq_expectations: [PinTransaction; 0] = [];
        let mut dreq = PinMock::new(&dreq_expectations);

        let mut driver = Vs1053Driver::new(spi_device, mp3cs, dreq).unwrap();

        let value = driver.sci_read(0x11).await.unwrap();
        // 0xAABB = 43707
        assert_eq!(value, 43707);

        let (mut spi_device, mut mp3cs, mut dreq) = driver.release();

        spi_device.done();
        mp3cs.done();

        dreq.done();
    }
}
