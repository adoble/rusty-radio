#![no_main]
#![no_std]
// Make std available when testing
//#![cfg_attr(not(test), no_std)]

//! An internet radio
//!

// [ ] Check out this about assigning resouces and rewriting initialized_peripherals.rs:
//      https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/assign_resources.rs

// See this about having functions to setup the peripherals and avoid the borrow problem:
// https://users.rust-lang.org/t/how-to-borrow-peripherals-struct/83565/2

// See rust-projects/edge-http-embassy-esp for how to access a web page using DNS.

mod async_delay;
mod constants;
use constants::NUMBER_SOCKETS_STACK_RESOURCES;

mod hardware;
use hardware::Hardware;

mod task;
use task::{
    //access_radio_stations::access_radio_stations,
    button_monitor::button_monitor,
    //display_web_content::display_web_content,
    play_music::play_music,
    read_test_music::read_test_music,
    stream::stream,
    sync::CODEC_DRIVER,
    wifi_tasks::{run_network_stack, wifi_connect},
};

// External crates
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, Output},
    spi::master::{Config as SpiConfig, Spi},
    time::RateExtU32,
};

use embassy_executor::Spawner;

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};

use embassy_time::{Duration, Timer};

use static_cell::StaticCell;

use async_delay::AsyncDelay;

static_assertions::const_assert!(true);

use vs1053_driver::Vs1053Driver;

static STACK: StaticCell<embassy_net::Stack> = StaticCell::new();

type SharedSpiBus = Mutex<NoopRawMutex, Spi<'static, esp_hal::Async>>;

type Vs1053DriverType<'a> = Vs1053Driver<
    SpiDeviceWithConfig<'a, NoopRawMutex, Spi<'a, esp_hal::Async>, Output<'a>>,
    Input<'a>,
    Output<'a>,
    AsyncDelay,
>;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::println!("Init!");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(72 * 1024); // TODO is this too big!

    // See this: https://github.com/esp-rs/esp-hal/blob/v0.21.1/esp-wifi/MIGRATING-0.9.md#memory-allocation
    //esp_alloc::heap_allocator!(92 * 1024);
    // esp_alloc::heap_allocator!(74 * 1024);

    // Initialise gpio ,spi and wifi peripherals
    let init_peripherals = Hardware::init::<NUMBER_SOCKETS_STACK_RESOURCES>(peripherals);

    let delay = AsyncDelay::new();

    // This is the way to initialize esp hal embassy for the the esp32c3
    // according to the example
    // https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point_with_sta.rs
    esp_hal_embassy::init(init_peripherals.system_timer.alarm0);

    // Need to convert the spi driver into an static blocking async version so that if can be accepted
    // by vs1053_driver::Vs1052Driver (which takes embedded_hal_async::spi::SpiDevice)
    static SPI_BUS: StaticCell<SharedSpiBus> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(init_peripherals.spi_bus));

    // The stack needs to be static so that it can be used in tasks.
    STACK.init(init_peripherals.sta_stack);

    // Init the vs1053 spi speeds
    let mut spi_sci_config = SpiConfig::default();
    spi_sci_config.frequency = 250.kHz();

    let mut spi_sdi_config = SpiConfig::default();
    spi_sdi_config.frequency = 8000.kHz();

    let spi_sci_device: SpiDeviceWithConfig<'_, NoopRawMutex, Spi<'_, esp_hal::Async>, Output<'_>> =
        SpiDeviceWithConfig::new(spi_bus, init_peripherals.xcs, spi_sci_config);
    let spi_sdi_device = SpiDeviceWithConfig::new(spi_bus, init_peripherals.xdcs, spi_sdi_config);

    let vs1053_driver: Vs1053DriverType = Vs1053Driver::new(
        spi_sci_device,
        spi_sdi_device,
        init_peripherals.dreq,
        init_peripherals.reset,
        delay,
    )
    .unwrap();

    // Setup the mutex for the vs1053 driver and then initialise the chip.
    {
        *(CODEC_DRIVER.lock().await) = Some(vs1053_driver);
        let mut driver_unlocked = CODEC_DRIVER.lock().await;
        if let Some(driver) = driver_unlocked.as_mut() {
            driver.begin().await.unwrap();
        }
    }

    // Print the registers using the shared driver for the vs1053
    print_registers().await;

    spawner
        .spawn(wifi_connect(init_peripherals.wifi_controller))
        .ok();

    esp_println::println!("wifi_connect should have been spawned");

    spawner
        .spawn(run_network_stack(init_peripherals.runner))
        .ok();
    spawner
        .spawn(button_monitor(init_peripherals.button_pin))
        .ok();
    // spawner
    //     .spawn(access_radio_stations(init_peripherals.sta_stack))
    // .ok();
    //spawner.spawn(display_web_content()).ok();

    //spawner.spawn(read_test_music()).ok();
    spawner.spawn(stream(init_peripherals.sta_stack)).ok();
    spawner.spawn(play_music()).ok();

    #[allow(deprecated)]
    spawner.spawn(notification_task()).ok();
}

#[deprecated(note = "Only used for development - Remove before release")]
#[embassy_executor::task]
async fn notification_task() {
    loop {
        Timer::after(Duration::from_millis(3_000)).await;
        esp_println::println!("Press button to access web!");
    }
}

async fn print_registers() {
    let mut driver_unlocked = CODEC_DRIVER.lock().await;
    if let Some(driver) = driver_unlocked.as_mut() {
        // Set the volume so we can see the value when we dump the registers
        let left_vol = 0x11;
        let right_vol = 0x22;

        driver.set_volume(left_vol, right_vol).await.unwrap();
        // Should see 1122 as the vol reg
        let registers = driver.dump_registers().await.unwrap();

        esp_println::println!("Dumped registers:");
        esp_println::println!("mode: {:X}", registers.mode);
        esp_println::println!("status: {:X}", registers.status);
        esp_println::println!("clockf: {:X}", registers.clock_f);
        esp_println::println!("volume: {:X}", registers.volume);
        esp_println::println!("audio_data : {:X}", registers.audio_data);
    } else {
        esp_println::println!("ERROR: Could not print registers");
    }
}

/// Used for testing.
#[deprecated(note = "Remove before release")]
#[embassy_executor::task]
async fn pulse_spi(mut driver: Vs1053DriverType<'static>) {
    let left_vol = 0x11;
    let right_vol = 0x22;

    loop {
        driver.set_volume(left_vol, right_vol).await.unwrap();

        Timer::after(Duration::from_micros(300)).await;
    }
}
