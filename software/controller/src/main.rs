//! embassy hello world
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

//% CHIPS: esp32 esp32c2 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3
//% FEATURES: embassy esp-hal-embassy/integrated-timers

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::{AnyPin, Input, Io, Level, Output, Pull};
use esp_hal::timer::timg::TimerGroup;

#[embassy_executor::task]
async fn run() {
    loop {
        esp_println::println!("Hello world from embassy using esp-hal-async!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}
// Blink something
#[embassy_executor::task]
async fn toggle_pin(mut pin: Output<'static, AnyPin>) {
    loop {
        pin.toggle();
        //esp_println::println!("Hello world from embassy using esp-hal-async!");
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task]
async fn button_monitor(mut pin: Input<'static, AnyPin>) {
    loop {
        pin.wait_for_falling_edge().await;
        esp_println::println!("Button pressed!");
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    esp_println::println!("Init!");

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let output_toggle_pin = Output::new(io.pins.gpio2, Level::High);
    let button_pin = Input::new(io.pins.gpio1, Pull::Up);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    spawner.spawn(run()).ok();
    spawner.spawn(toggle_pin(output_toggle_pin)).ok();
    spawner.spawn(button_monitor(button_pin)).ok();

    loop {
        esp_println::println!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}
