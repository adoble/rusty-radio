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

// NEED TO SET THE SPEED OF THE SPI EXTERNALLY!

use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;
use embedded_hal_async::spi::{Operation, SpiDevice};

//use embedded_hal::digital::OutputPin;

mod registers;

//use embedded_hal_bus::spi::DeviceError;
use registers::{Mode, Register};

const SCI_READ: u8 = 0b0000_0011;
const SCI_WRITE: u8 = 0b0000_0010;

pub struct Vs1053Driver<SPI, DREQ, RST, DLY> {
    spi_control_device: SPI,
    spi_data_device: SPI,
    dreq: DREQ,
    reset: RST,
    delay: DLY,
}

impl<SPI, DREQ, RST, DLY> Vs1053Driver<SPI, DREQ, RST, DLY>
where
    SPI: SpiDevice,
    DREQ: Wait, // See https://docs.rs/embedded-hal-async/1.0.0/embedded_hal_async/digital/index.html
    RST: OutputPin,
    DLY: DelayNs,
{
    pub fn new(
        spi_control_device: SPI,
        spi_data_device: SPI,
        dreq: DREQ,
        reset: RST,
        delay: DLY,
    ) -> Result<Self, DriverError> {
        let driver = Vs1053Driver {
            spi_control_device,
            spi_data_device,
            dreq,
            reset,
            delay,
        };
        Ok(driver)
    }

    pub async fn reset(&self) -> Result<(), DriverError> {
        Ok(())
    }

    /// The should be called during the initialisation of the program, i.e. after the power
    /// has come up.

    pub async fn begin(&mut self) -> Result<(), DriverError> {
        self.reset.set_high().map_err(|_| DriverError::Reset)?;

        self.reset_device().await?;

        // TODO add this as in the adafruit driver
        // return (sciRead(VS1053_REG_STATUS) >> 4) & 0x0F;

        Ok(())
    }

    /// Reset the device.
    /// Assumes that the clock frequency is 12.288 MHz.
    pub async fn reset_device(&mut self) -> Result<(), DriverError> {
        self.reset.set_low().map_err(|_| DriverError::Reset)?;
        self.delay.delay_ms(100).await;
        self.reset.set_high().map_err(|_| DriverError::Reset)?;

        // From data sheet: After a hardware reset (or at power-up) DREQ will stay
        // down for around 22000 clock cycles, which means an approximate 1.8 ms
        // delay if VS1053b is run at 12.288 MHz.
        // Rather than using a delay, just wait until DREQ has gone high
        self.dreq
            .wait_for_high()
            .await
            .map_err(|_| DriverError::DReq)?;

        // TODO do we need to do this. Experiment with taking it away!
        self.soft_reset().await?;

        // Set the clock divider. This has to be done as soon as pssible after a soft reset
        // TODO change "soft reset" to "software reset" as this is the name in the data sheet
        self.sci_write(Register::Clockf.into(), 0x6000).await?;

        // Set volume to a confortable level
        self.set_volume(40, 40).await?;

        // TODO bass leveb

        Ok(())
    }

    async fn soft_reset(&mut self) -> Result<(), DriverError> {
        self.sci_write(Register::Mode.into(), Mode::SdiNew | Mode::Reset)
            .await?;

        self.delay.delay_us(2).await;

        self.dreq
            .wait_for_high()
            .await
            .map_err(|_| DriverError::DReq)?;

        Ok(())
    }

    pub async fn set_volume(&mut self, left: u8, right: u8) -> Result<(), DriverError> {
        let volume = ((right as u16) << 8) | left as u16;

        self.sci_write(Register::Volume.into(), volume).await
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
    pub fn release(self) -> (SPI, SPI, DREQ, RST, DLY) {
        (
            self.spi_control_device,
            self.spi_data_device,
            self.dreq,
            self.reset,
            self.delay,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DriverError {
    SpiRead,
    SpiWrite,
    // An error in waiting for the DREQ signal
    DReq,
    // An error in setting the reset pin
    Reset,
}

#[cfg(test)]
mod tests {
    use super::*;

    use embedded_hal_mock::eh1::delay::NoopDelay;
    use embedded_hal_mock::eh1::digital::{Mock as PinMock, State, Transaction as PinTransaction};
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

        let dreq_expectations = [PinTransaction::wait_for_state(State::High)];
        let dreq = PinMock::new(&dreq_expectations);

        let reset_expectations: [PinTransaction; 0] = [];
        let reset = PinMock::new(&reset_expectations);

        let delay = NoopDelay::new();

        let mut driver =
            Vs1053Driver::new(spi_control_device, spi_data_device, dreq, reset, delay).unwrap();

        let value = driver.sci_read(0x11).await.unwrap();
        // 0xAABB = 43707
        assert_eq!(value, 43707);

        let (mut spi_control_device, mut spi_data_device, mut dreq, mut reset, mut _delay) =
            driver.release();

        spi_control_device.done();
        spi_data_device.done();
        dreq.done();
        reset.done();
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

        let reset_expectations: [PinTransaction; 0] = [];
        let reset = PinMock::new(&reset_expectations);

        let delay = NoopDelay::new();

        let mut driver =
            Vs1053Driver::new(spi_control_device, spi_data_device, dreq, reset, delay).unwrap();

        // 0xAABB = 43707
        driver.sci_write(0x11, 43707).await.unwrap();

        let (mut spi_control_device, mut spi_data_device, mut dreq, mut reset, mut _delay) =
            driver.release();

        spi_control_device.done();
        spi_data_device.done();
        dreq.done();
        reset.done();
    }

    #[async_std::test]
    async fn volume_test() {
        let spi_data_device = SpiMock::new(&[]);
        let reset = PinMock::new(&[]);
        let delay = NoopDelay::new();

        // Volume 40, 40 =  0x2828
        let spi_control_expectations = [
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![SCI_WRITE, 0x0B, 0x28, 0x28]),
            SpiTransaction::transaction_end(),
        ];
        let spi_control_device = SpiMock::new(&spi_control_expectations);

        let dreq_expectations = [PinTransaction::wait_for_state(State::High)];
        let dreq = PinMock::new(&dreq_expectations);

        let mut driver =
            Vs1053Driver::new(spi_control_device, spi_data_device, dreq, reset, delay).unwrap();

        driver.set_volume(40, 40).await.unwrap();

        let (mut spi_control_device, mut spi_data_device, mut dreq, mut reset, mut _delay) =
            driver.release();

        spi_control_device.done();
        spi_data_device.done();
        dreq.done();
        reset.done();
    }
}
