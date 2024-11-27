#![cfg_attr(not(test), no_std)]

use embedded_hal_async::digital::Wait;
use embedded_hal_async::spi::{Operation, SpiDevice};

use embedded_hal::digital::OutputPin;

const READ: u8 = 0b0000_0011;

pub struct Vs1053Driver<SPI, MP3CS, XDCS, DREQ> {
    spi: SPI,
    mp3cs: MP3CS,
    xdcs: XDCS,
    dreq: DREQ,
}

impl<SPI, MP3CS, XDCS, DREQ> Vs1053Driver<SPI, MP3CS, XDCS, DREQ>
where
    SPI: SpiDevice,
    MP3CS: OutputPin,
    XDCS: OutputPin,
    DREQ: Wait, // See https://docs.rs/embedded-hal-async/1.0.0/embedded_hal_async/digital/index.html
{
    pub fn new(spi: SPI, mp3cs: MP3CS, xdcs: XDCS, dreq: DREQ) -> Result<Self, DriverError> {
        let driver = Vs1053Driver {
            spi,
            mp3cs,
            xdcs,
            dreq,
        };
        Ok(driver)
    }

    pub async fn sci_read(&mut self, addr: u8, data: &mut [u8]) -> Result<(), DriverError> {
        // XDCS needs to be set low. This should be done by the transaction
        // Send read instruction (0b0000 0011)
        // then address (u8)
        // then read data (2 x u8)

        //let mut buf = [0; 2];

        self.spi
            .transaction(&mut [Operation::Write(&[READ, addr]), Operation::Read(data)])
            .await
            .map_err(|_| DriverError::SpiRead)?;

        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn sci_write(&mut self, _addr: u8, _data: u16) -> Result<(), DriverError> {
        todo!()
    }

    // Destroys the driver and relases the  peripherals
    pub fn release(self) -> (SPI, MP3CS, XDCS, DREQ) {
        (self.spi, self.mp3cs, self.xdcs, self.dreq)
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

    use embedded_hal_mock::eh1::digital::{
        Mock as PinMock, State as PinState, Transaction as PinTransaction,
    };
    use embedded_hal_mock::eh1::spi::{Mock as SpiMock, Transaction as SpiTransaction};

    #[async_std::test]
    //#[embassy_executor::test]
    async fn sci_read_test() {
        let spi_expectations = [
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![READ, 0x11]),
            SpiTransaction::read_vec(vec![0xAA, 0xBB]),
            SpiTransaction::transaction_end(),
        ];
        let spi = SpiMock::new(&spi_expectations);

        //let xdcs_expectations = [PinTransaction::set(PinState::Low)];
        let xdcs_expectations: [PinTransaction; 0] = [];
        let xdcs = PinMock::new(&xdcs_expectations);
        // let mp3cs_expectations = [
        //     PinTransaction::set(PinState::Low),
        //     PinTransaction::set(PinState::High),
        // ];
        let mp3cs_expectations: [PinTransaction; 0] = [];
        let mp3cs = PinMock::new(&xdcs_expectations);

        // let dreq_expectations = [
        //     PinTransaction::get(PinState::High),
        //     PinTransaction::get(PinState::Low),
        // ];
        let dreq_expectations: [PinTransaction; 0] = [];
        let mut dreq = PinMock::new(&dreq_expectations);

        let mut driver = Vs1053Driver::new(spi, mp3cs, xdcs, dreq).unwrap();

        let mut buf: [u8; 2] = [0; 2];
        let _ = driver.sci_read(0x11, &mut buf).await.unwrap();

        let (mut spi, mut mp3cs, mut xdcs, mut dreq) = driver.release();

        spi.done();
        mp3cs.done();
        xdcs.done();
        dreq.done();
    }
}
