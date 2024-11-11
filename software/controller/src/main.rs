#![no_std]
#![no_main]

use core::sync::atomic::Ordering;
use embassy_executor::{Spawner};
use embassy_time::{Duration, Timer};
use embedded_hal_async::digital::Wait;
use esp_backtrace as _;
//use esp_hal::gpio::{AnyPin, Input, PullUp};
use esp_hal::gpio::{AnyPin, Input};
//use esp_hal::{clock::ClockControl, embassy, peripherals::Peripherals, prelude::*, IO};
use portable_atomic::AtomicU32;

use esp_hal::{delay::Delay, prelude::*};

#[embassy_executor::main]
async fn main(spawner: Spawner)
{
    #[allow(unused)]
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    esp_println::logger::init_logger_from_env();

    loop {
        log::info!("Hello world!");
        delay.delay(500.millis());
    }
}
