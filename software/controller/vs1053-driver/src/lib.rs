#![cfg_attr(not(test), no_std)]

//! Based on the Adafruit driver  https://github.com/adafruit/Adafruit_VS1053_Library/tree/master

//! This driver uses no chip select (xdcs or xcs) pins as this is managed by `SpiDevice`s.
//! If the hal only provides an `SpiBus` then see [this](https://github.com/rust-embedded/embedded-hal/blob/master/docs/migrating-from-0.2-to-1.0.md#for-end-users) on how to convert a `SpiBus`
//! into a `SpiDevice`
//!

// Note: The VS1053 uses a high speed and low speed SPI connnection.
// See https://docs.esp-rs.org/esp-hal/esp-hal/0.22.0/esp32c3/esp_hal/spi/master/index.html#shared-spi-access
// and  https://docs.embassy.dev/embassy-embedded-hal/git/default/shared_bus/asynch/spi/index.html
// for hints on how to set this up, but with the same SPI peripheral.
// Also look at embassy_embedded_hal::shared_bus::asynch::spi:SpiDeviceWithConfig (see https://docs.embassy.dev/embassy-embedded-hal/git/default/shared_bus/asynch/spi/struct.SpiDeviceWithConfig.html)
// to see how to individually configure the speed of a SpiDevice.

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

mod dump_registers;
mod registers;

//use embedded_hal_bus::spi::DeviceError;
use dump_registers::DumpRegisters;
use registers::{Mode, Register};

const SCI_READ: u8 = 0b0000_0011;
const SCI_WRITE: u8 = 0b0000_0010;

/// The amount of coded data sent before checking if
/// the VS1053 can accept more data
const DATA_CHUNK_SIZE: usize = 32;

pub struct Vs1053Driver<SPI, DREQ, RST, DLY> {
    spi_control_device: SPI,
    spi_data_device: SPI,
    dreq: DREQ,
    reset: RST,
    delay: DLY,
}

// TODO about the delay - see https://embassy.dev/book/#_delay
// Maybe don't need to the bridging code in AsyncDelay
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

    // TODO should this go into new()?
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

        self.soft_reset().await?;

        // Set the clock divider. This has to be done as soon as pssible after a soft reset
        // TODO change "soft reset" to "software reset" as this is the name in the data sheet
        self.sci_write(Register::Clockf.into(), 0x6000).await?;

        // Set volume to a comfortable level
        let left = 0x28; // Dec 40
        let right = 0x28; // Dec 40
        self.set_volume(left, right).await?;

        // TODO bass level

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

    pub async fn play_data(&mut self, buffer: &[u8]) -> Result<(), DriverError> {
        for chunk in buffer.chunks(DATA_CHUNK_SIZE) {
            self.dreq
                .wait_for_high()
                .await
                .map_err(|_| DriverError::DReq)?;

            self.spi_data_device
                .transaction(&mut [Operation::Write(chunk)])
                .await
                .map_err(|_| DriverError::SpiDataWrite)?;
        }

        Ok(())
    }

    pub async fn set_volume(&mut self, left: u8, right: u8) -> Result<(), DriverError> {
        let volume = ((left as u16) << 8) | right as u16;

        self.sci_write(Register::Volume.into(), volume).await
    }

    pub async fn sample_rate(&mut self) -> Result<u16, DriverError> {
        let reg_value = self.sci_read(Register::AudioData.into()).await?;

        // Sample rate/2 held in bits 15:1
        let sample_rate = reg_value & 0xFFFE;

        Ok(sample_rate)
    }

    /// Dumps the values of selected registers into a `DumpRegisters` structure.
    /// This function is only used for debugging!
    pub async fn dump_registers(&mut self) -> Result<DumpRegisters, DriverError> {
        let mode = self.sci_read(Register::Mode.into()).await?;
        let status = self.sci_read(Register::Status.into()).await?;
        let clock_f = self.sci_read(Register::Clockf.into()).await?;
        let volume = self.sci_read(Register::Volume.into()).await?;
        let audio_data = self.sci_read(Register::AudioData.into()).await?;

        let dr = DumpRegisters {
            mode,
            status,
            clock_f,
            volume,
            audio_data,
        };

        Ok(dr)
    }

    // This is the old sine test. TODO update to new one
    /// Duration in milliseconds
    /// n is the test. See the spec section 10.12.1
    /// But n = 126 gives a sine frequency of 5168Hz.
    pub async fn sine_test(&mut self, n: u8, duration: u32) -> Result<(), DriverError> {
        self.reset().await?;

        let mut mode = self.sci_read(Register::Mode.into()).await?;
        mode |= 0x0020;
        self.sci_write(Register::Mode.into(), mode).await?;

        self.dreq
            .wait_for_high()
            .await
            .map_err(|_| DriverError::DReq)?;

        let sine_start: [u8; 8] = [0x53, 0xEF, 0x6E, n, 0x00, 0x00, 0x00, 0x00];
        let sine_stop: [u8; 8] = [0x45, 0x78, 0x69, 0x74, 0x00, 0x00, 0x00, 0x00];

        self.spi_control_device
            .transaction(&mut [Operation::Write(&sine_start)])
            .await
            .map_err(|_| DriverError::SpiControlWrite)?;

        self.delay.delay_ms(duration).await;

        self.spi_control_device
            .transaction(&mut [Operation::Write(&sine_stop)])
            .await
            .map_err(|_| DriverError::SpiControlWrite)?;

        Ok(())
    }

    /// Sweep test.
    /// Note that this is a very slow sweep through all frequencies
    /// and therefore it can take sometine before the human ear can
    /// hear something.
    ///
    /// See VS1053 data sheet Section 10.12.2
    pub async fn sweep_test(&mut self) -> Result<(), DriverError> {
        self.reset().await?;

        // Set test mode
        let mut mode = self.sci_read(Register::Mode.into()).await?;
        mode |= 0x0020;
        self.sci_write(Register::Mode.into(), mode).await?;

        self.dreq
            .wait_for_high()
            .await
            .map_err(|_| DriverError::DReq)?;

        self.sci_write(Register::AaiAddr.into(), 0x4022).await?;

        // TODO reset test mode!

        Ok(())
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
            .map_err(|_| DriverError::SpiControlRead)?;

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
            .map_err(|_| DriverError::SpiControlWrite)?;

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
    /// An error reading control (SCI) data
    SpiControlRead,
    /// An error wrinting control (SCI) data
    SpiControlWrite,
    /// An error writing coded data (SDA) to the codec
    SpiDataWrite,
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

    #[async_std::test]
    async fn dump_registers_test() {
        let spi_control_expectations = [
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![SCI_READ, Register::Mode.into()]),
            SpiTransaction::read_vec(vec![0xAB, 0xCD]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![SCI_READ, Register::Status.into()]),
            SpiTransaction::read_vec(vec![0xAB, 0xCD]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![SCI_READ, Register::Clockf.into()]),
            SpiTransaction::read_vec(vec![0x98, 0x76]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![SCI_READ, Register::Volume.into()]),
            SpiTransaction::read_vec(vec![0xAB, 0xCD]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![SCI_READ, Register::AudioData.into()]),
            SpiTransaction::read_vec(vec![0x60, 0x00]),
            SpiTransaction::transaction_end(),
        ];
        let spi_control_device = SpiMock::new(&spi_control_expectations);

        let spi_data_expectations: [SpiTransaction<u8>; 0] = [];
        let spi_data_device = SpiMock::new(&spi_data_expectations);

        let dreq_expectations = [
            PinTransaction::wait_for_state(State::High),
            PinTransaction::wait_for_state(State::High),
            PinTransaction::wait_for_state(State::High),
            PinTransaction::wait_for_state(State::High),
            PinTransaction::wait_for_state(State::High),
        ];
        let dreq = PinMock::new(&dreq_expectations);

        let reset_expectations: [PinTransaction; 0] = [];
        let reset = PinMock::new(&reset_expectations);

        let delay = NoopDelay::new();

        let mut driver =
            Vs1053Driver::new(spi_control_device, spi_data_device, dreq, reset, delay).unwrap();

        let result_dr = driver.dump_registers().await.unwrap();

        assert_eq!(
            DumpRegisters {
                mode: 0xABCD,
                status: 0xABCD,
                clock_f: 0x9876,
                volume: 0xABCD,
                audio_data: 0x6000
            },
            result_dr
        );

        let (mut spi_control_device, mut spi_data_device, mut dreq, mut reset, mut _delay) =
            driver.release();

        spi_control_device.done();
        spi_data_device.done();
        dreq.done();
        reset.done();
    }

    #[async_std::test]
    async fn sample_rate_test() {
        let spi_control_expectations = [
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![SCI_READ, Register::AudioData.into()]),
            SpiTransaction::read_vec(vec![0xAC, 0x45]),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(vec![SCI_READ, Register::AudioData.into()]),
            SpiTransaction::read_vec(vec![0x2B, 0x10]),
            SpiTransaction::transaction_end(),
        ];
        let spi_control_device = SpiMock::new(&spi_control_expectations);

        let spi_data_expectations: [SpiTransaction<u8>; 0] = [];
        let spi_data_device = SpiMock::new(&spi_data_expectations);

        let dreq_expectations = [
            PinTransaction::wait_for_state(State::High),
            PinTransaction::wait_for_state(State::High),
        ];
        let dreq = PinMock::new(&dreq_expectations);

        let reset_expectations: [PinTransaction; 0] = [];
        let reset = PinMock::new(&reset_expectations);

        let delay = NoopDelay::new();

        let mut driver =
            Vs1053Driver::new(spi_control_device, spi_data_device, dreq, reset, delay).unwrap();

        let sample_rate = driver.sample_rate().await.unwrap();

        assert_eq!(44100, sample_rate);

        let sample_rate = driver.sample_rate().await.unwrap();
        assert_eq!(11024, sample_rate);

        let (mut spi_control_device, mut spi_data_device, mut dreq, mut reset, mut _delay) =
            driver.release();

        spi_control_device.done();
        spi_data_device.done();
        dreq.done();
        reset.done();
    }

    #[async_std::test]
    async fn play_data_test() {
        let test_data_chunk_1 = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ];

        let test_data_chunk_2 = vec![
            101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117,
            118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132,
        ];

        let test_data_chunk_3 = vec![201, 202, 203, 204, 205];

        let mut all_test_data: Vec<u8> = Vec::new();
        all_test_data.extend(test_data_chunk_1.iter());
        all_test_data.extend(test_data_chunk_2.iter());
        all_test_data.extend(test_data_chunk_3.iter());

        let test_buffer = all_test_data.as_slice();
        assert_eq!(test_data_chunk_1.len(), 32);

        let spi_data_expectations = [
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(test_data_chunk_1),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(test_data_chunk_2),
            SpiTransaction::transaction_end(),
            SpiTransaction::transaction_start(),
            SpiTransaction::write_vec(test_data_chunk_3),
            SpiTransaction::transaction_end(),
        ];

        let spi_control_expectations: [SpiTransaction<u8>; 0] = [];

        let spi_data_device = SpiMock::new(&spi_data_expectations);
        let spi_control_device = SpiMock::new(&spi_control_expectations);

        let dreq_expectations = [
            PinTransaction::wait_for_state(State::High),
            PinTransaction::wait_for_state(State::High),
            PinTransaction::wait_for_state(State::High),
        ];
        let dreq = PinMock::new(&dreq_expectations);

        let reset_expectations: [PinTransaction; 0] = [];
        let reset = PinMock::new(&reset_expectations);

        let delay = NoopDelay::new();

        let mut driver =
            Vs1053Driver::new(spi_control_device, spi_data_device, dreq, reset, delay).unwrap();

        driver.play_data(test_buffer).await.unwrap();

        let (mut spi_control_device, mut spi_data_device, mut dreq, mut reset, mut _delay) =
            driver.release();

        spi_control_device.done();
        spi_data_device.done();
        dreq.done();
        reset.done();
    }
}
