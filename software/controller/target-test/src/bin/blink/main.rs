#![no_main]
#![no_std]

use embassy_executor::Spawner;
use embassy_time::Timer;

use esp_backtrace as _;

use esp_hal::gpio::{Level, Output};
use esp_hal::prelude::*;
use esp_hal::timer::timg::TimerGroup;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::println!("Starting ");

    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    //esp_alloc::heap_allocator!(72 * 1024); // TODO is this too big!

    //let button_pin = Input::new(peripherals.GPIO1, Pull::Up);
    let led_pin = Output::new(peripherals.GPIO2, Level::High);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    spawner.must_spawn(blink(led_pin));
}

#[embassy_executor::task]
async fn blink(mut led_pin: Output<'static>) {
    loop {
        esp_println::println!("Blink");
        led_pin.toggle();
        Timer::after_millis(250).await;
    }
}
