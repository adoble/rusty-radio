//! The VS1053 driver uses the `embedded_hal_async::delay::DelayNs` type to setup
//! delays. This is not natively available in Embassy so this struct provides a
//! bridge between the `embassy-time` features and the embedded-hal Delay
//! use embedded_hal_async::delay::DelayNs;
use embassy_time::{Duration, Timer};
use embedded_hal_async::delay::DelayNs;

pub struct AsyncDelay;

impl AsyncDelay {
    pub fn new() -> Self {
        AsyncDelay
    }
}

impl DelayNs for AsyncDelay {
    async fn delay_ns(&mut self, ns: u32) {
        let ms = ns / 1_000_000; // Convert nanoseconds to milliseconds
        Timer::after(Duration::from_millis(ms as u64)).await;
    }
}
