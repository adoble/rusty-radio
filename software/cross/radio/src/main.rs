#![cfg_attr(not(test), no_std)]
#![no_main]

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
    command_processor::process_commands,
    play_music::play_music,
    stream::stream,
    sync::CODEC_DRIVER,
    sync::STATION_CHANGE_WATCH,
    wifi_tasks::{run_network_stack, wifi_connect},
};

mod radio_stations;
use radio_stations::{radio_stations, RadioStations};

// mod radio_errorX;
// use radio_errorX::RadioError;

//use stations::{Station, Stations};

// External crates
//use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, Output},
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
    uart::Uart,
    Async,
};

use embassy_executor::Spawner;

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;
//use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig as BlockingSpiDeviceWithConfig;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

use static_cell::StaticCell;

use async_delay::AsyncDelay;

static_assertions::const_assert!(true);

use vs1053_driver::Vs1053Driver;

use mcp23s17_async::Mcp23s17;

// use ra8875::RA8875;

use log::*;

use esp_alloc as _;

use crate::{constants::STATIONS_URL, hardware::WifiHardware, radio_stations::RadioStationsError};

static STACK: StaticCell<embassy_net::Stack> = StaticCell::new();

type SharedSpiBus = Mutex<CriticalSectionRawMutex, Spi<'static, esp_hal::Async>>;

// type MultiplexerDriverType<'a> =
//     SpiDeviceWithConfig<'a, CriticalSectionRawMutex, Spi<'a, esp_hal::Async>, Output<'a>>;

type Vs1053DriverType<'a> = Vs1053Driver<
    SpiDeviceWithConfig<'a, CriticalSectionRawMutex, Spi<'a, esp_hal::Async>, Output<'a>>,
    Input<'a>,
    Output<'a>,
    AsyncDelay,
>;

pub type MultiplexerDriverType<'a> =
    Mcp23s17<SpiDeviceWithConfig<'a, CriticalSectionRawMutex, Spi<'a, esp_hal::Async>, Output<'a>>>;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

// #[esp_hal_embassy::main]
#[esp_rtos::main]
async fn main(spawner: Spawner) {
    #[cfg(not(feature = "no_log"))]
    esp_println::logger::init_logger(log::LevelFilter::Debug);

    info!("Rusty Radio started");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // See this: https://github.com/esp-rs/esp-hal/blob/v0.21.1/esp-wifi/MIGRATING-0.9.md#memory-allocation
    // Size has been empirically determined.
    // esp_alloc::heap_allocator!(size: 48 * 1024); //Recommended
    esp_alloc::heap_allocator!(size: 76 * 1024); // This worked

    // Initialise gpio ,spi and wifi peripherals. The initialised peripherals are then fields in the hardware struct
    // and are given symbolic names.
    let mut hardware = Hardware::init::<NUMBER_SOCKETS_STACK_RESOURCES>(peripherals);

    let delay = AsyncDelay::new();

    // Initialise the schedular (in this case embasssy).
    // This needs to be done before initialiing the wifi
    esp_rtos::start(hardware.system_timer.alarm0, hardware.software_interrupt0);
    //esp_rtos::start(hardware.system_timer.alarm0);

    let wifi_hardware =
        WifiHardware::init_wifi::<NUMBER_SOCKETS_STACK_RESOURCES>(hardware.wifi, hardware.rng);

    // The stack needs to be static so that it can be used in tasks.
    STACK.init(wifi_hardware.sta_stack);

    // Need to convert the spi driver into an static blocking async version so that if can be accepted
    // by vs1053_driver::Vs1052Driver (which takes embedded_hal_async::spi::SpiDevice)
    static SPI_BUS: StaticCell<SharedSpiBus> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(hardware.spi_bus));

    /*
       ---------------------------
       Setup the VS1053 MP3 codec
       ---------------------------
    */
    // Init the vs1053 spi speeds
    // spi_sci_config.frequency = Rate::from_khz(250); //250.kHz();
    let spi_sci_config = SpiConfig::default().with_frequency(Rate::from_khz(250));

    // spi_sdi_config.frequency = Rate::from_khz(8000); //  8000.kHz();
    let spi_sdi_config = SpiConfig::default().with_frequency(Rate::from_khz(8000));

    let spi_sci_device = SpiDeviceWithConfig::new(spi_bus, hardware.xcs, spi_sci_config);
    let spi_sdi_device = SpiDeviceWithConfig::new(spi_bus, hardware.xdcs, spi_sdi_config);

    let vs1053_driver: Vs1053DriverType = Vs1053Driver::new(
        spi_sci_device,
        spi_sdi_device,
        hardware.dreq,
        hardware.reset_codec,
        delay,
    )
    .unwrap(); //TODO error handling

    // Setup the mutex for the vs1053 driver and then initialise the chip.
    {
        *(CODEC_DRIVER.lock().await) = Some(vs1053_driver);
        let mut driver_unlocked = CODEC_DRIVER.lock().await;
        if let Some(driver) = driver_unlocked.as_mut() {
            driver.begin().await.unwrap();
        }
    }

    // The uart is passed to a task so make it static
    let uart = {
        static UART: StaticCell<Uart<'_, Async>> = StaticCell::new();
        UART.init(hardware.uart)
    };

    /*
       ----------------------------------------
       Setup channel publishers and subscribers
       ----------------------------------------
    */
    // // Set up the end points of the radio commands channel. Setting the publishers and subscriber up this way to avoid borrow
    // // problems when using the same publisher and subscriber for the (later spawned) tasks.
    // // SAFETY: These calls fail only if the number of subscribers and publishers have been incorrectly set up
    // // in RADIO_COMMAND_CHANNEL.This is then a programming error and should result in a panic, using unwrap here.
    // let radio_command_pub: RadioCommandChannelPublisher =
    //     RADIO_COMMAND_CHANNEL.publisher().unwrap();
    // //let radio_command_sub = RADIO_COMMAND_CHANNEL.subscriber().unwrap();

    // Setting up the network
    spawner
        .spawn(wifi_connect(wifi_hardware.wifi_controller))
        .ok();
    spawner.spawn(run_network_stack(wifi_hardware.runner)).ok();

    // Check that the link signal is present and the stack has a valid IP config
    // before continuing.
    wifi_hardware.sta_stack.wait_link_up().await;
    wifi_hardware.sta_stack.wait_config_up().await;

    // Read the stations from the web. Note that this also signals the
    // initial station to other tasks.
    // TODO improve the error handling
    let stations = radio_stations(
        wifi_hardware.sta_stack,
        STATIONS_URL,
        STATION_CHANGE_WATCH.sender(),
    )
    .await
    .unwrap_or_else(|e| {
        panic!("Could not read radio stations: {:?}", e);
    });

    //.expect("Could not read radio stations.");
    static RADIO_STATIONS: StaticCell<RadioStations> = StaticCell::new();
    // let radio_stations: &mut stations::Stations<_, _, _> = RADIO_STATIONS.init(statios);
    let radio_stations: &mut RadioStations = RADIO_STATIONS.init(stations);

    // Streaming and playing music
    spawner.spawn(stream(wifi_hardware.sta_stack)).ok();

    spawner.spawn(play_music()).ok();

    // Process the serial commands comming from the UI processor
    spawner
        .spawn(process_commands(
            uart,
            STATION_CHANGE_WATCH.sender(),
            radio_stations,
        ))
        .ok();

    // Signal the UI that the radio is ready
    hardware.radio_ready.set_low();

    // Showing on the panel LED when a station has been tuned in
    // TODO This is a temporary solution until the display is ready.
    //spawner.spawn(station_indicator(front_panel)).ok();
}
