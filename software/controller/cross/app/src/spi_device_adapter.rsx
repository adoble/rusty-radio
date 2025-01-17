use embassy_sync::blocking_mutex::raw::RawMutex;
use embedded_hal::digital::ErrorType;
use embedded_hal::digital::OutputPin;
//use embedded_hal::spi::SpiDevice;
//use embedded_hal_async::shared_bus::asynch::SpiDeviceWithConfig;
use embedded_hal_async::spi::Operation;
use embedded_hal_async::spi::SpiDevice;

// Wrapped types
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;

pub struct SpiDeviceAdapter<T> {
    wrapped: T,
}

impl<T> SpiDeviceAdapter<T> {
    pub fn new(wrapped: T) -> Self {
        Self { wrapped }
    }
}

impl<T, E> embedded_hal_async::spi::ErrorType for SpiDeviceAdapter<T>
where
    // E: embedded_hal::spi::Error,
    E: embedded_hal_async::spi::Error,
    T: SpiDevice<u8, Error = E>,
{
    type Error = E;
}

//impl<T, E> embedded_hal_async::spi::SpiDevice<u8> for SpiDeviceAdapter<T>
//impl<T> embedded_hal_async::spi::SpiDevice<u8> for SpiDeviceAdapter<T>

//embassy_embedded_hal::shared_bus::asynch::spi
//pub struct SpiDeviceWithConfig<'a, M, BUS, CS>

impl<T> embedded_hal_async::spi::SpiDevice for SpiDeviceAdapter<T>
where
    //E: embedded_hal_async::spi::Error + 'static,
    // T: blocking::spi::Transfer<u8, Error = E> + blocking::spi::Write<u8, Error = E>,
    T: SpiDeviceWithConfig<'_, NoopRawMutex, Spi<'_, esp_hal::Async>, Output<'_>>,
    WORD: u8,
{
    async fn transaction(
        &mut self,
        operations: &mut [Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        self.wrapped.transaction(operations).await?;
        Ok(())
    }
}
