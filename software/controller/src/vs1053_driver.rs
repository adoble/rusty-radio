use embedded_hal_async::digital::Wait;
use embedded_hal_async::spi::SpiDevice;
use esp_hal::gpio::OutputPin;

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
}

pub enum DriverError {}
