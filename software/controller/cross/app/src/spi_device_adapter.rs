//use embedded_hal::spi::SpiDevice;
//use embedded_hal_async::shared_bus::asynch::SpiDeviceWithConfig;
use embedded_hal_async::spi::Operation;
use embedded_hal_async::spi::SpiDevice;

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
impl<T> embedded_hal_async::spi::SpiDevice<u8> for SpiDeviceAdapter<T>
where
    //E: embedded_hal_async::spi::Error + 'static,
    // T: blocking::spi::Transfer<u8, Error = E> + blocking::spi::Write<u8, Error = E>,
    T: embedded_hal_async::spi::SpiDevice<u8>,
{
    async fn transaction(
        &mut self,
        operations: &mut [Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        self.wrapped.transaction(operations).await?;
        Ok(())
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.transaction(&mut [Operation::Read(buf)]).await
    }

    async fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.transaction(&mut [Operation::Write(buf)]).await
    }

    async fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        self.transaction(&mut [Operation::Transfer(read, write)])
            .await
    }

    async fn transfer_in_place(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.transaction(&mut [Operation::TransferInPlace(buf)])
            .await
    }
}
