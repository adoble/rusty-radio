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
    play_music::play_music,
    //read_test_music::read_test_music,
    stream::stream,
    //stream2::stream2,
    sync::CODEC_DRIVER,
    wifi_connected_indicator::wifi_connected_indicator,
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

//use embassy_time::{Duration, Timer};

use static_cell::StaticCell;

use async_delay::AsyncDelay;

static_assertions::const_assert!(true);

use stations::{Station, StationError, Stations};
use vs1053_driver::Vs1053Driver;
static STACK: StaticCell<embassy_net::Stack> = StaticCell::new();

type SharedSpiBus = Mutex<NoopRawMutex, Spi<'static, esp_hal::Async>>;

type Vs1053DriverType<'a> = Vs1053Driver<
    SpiDeviceWithConfig<'a, NoopRawMutex, Spi<'a, esp_hal::Async>, Output<'a>>,
    Input<'a>,
    Output<'a>,
    AsyncDelay,
>;

// TODO All the hard coded stations have to be made variable.
// NOTE: This station does a number of redirects by setting the response header "location". Note that it does
// not give a return code 3xx which is strange.
// Anaylsed with Google HAR analyser https://toolbox.googleapps.com/apps/har_analyzer/
// For a description of the location field see: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Location
const STATION_URL: &str = "http://liveradio.swr.de/sw282p3/swr3/play.mp3";

// NOTE: This station doesn't seem to have redirects (as of now) so used to test the basic functionality
//const STATION_URL: &str = "http://listen.181fm.com/181-classical_128k.mp3";

// Local server for testing
//const STATION_URL: &str = "http://192.168.2.107:8080/music/2"; // Hijo de la Luna. 128 kb/s

const STATION: (&str, &str) = ("SWR3", "http://liveradio.swr.de/sw282p3/swr3/play.mp3");

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::println!("Rusty Radio started");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // See this: https://github.com/esp-rs/esp-hal/blob/v0.21.1/esp-wifi/MIGRATING-0.9.md#memory-allocation
    //esp_alloc::heap_allocator!(72 * 1024); // This value works!
    esp_alloc::heap_allocator!(76 * 1024); // TODO is this too big!

    //esp_alloc::heap_allocator!(48 * 1024);   //Recommanded

    // Initialise gpio ,spi and wifi peripherals. The initialised peripherals are then fields in the hardware struct.
    let mut hardware = Hardware::init::<NUMBER_SOCKETS_STACK_RESOURCES>(peripherals);

    let delay = AsyncDelay::new();

    // This is the way to initialize esp hal embassy for the the esp32c3
    // according to the example
    // https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point_with_sta.rs
    esp_hal_embassy::init(hardware.system_timer.alarm0);

    // Need to convert the spi driver into an static blocking async version so that if can be accepted
    // by vs1053_driver::Vs1052Driver (which takes embedded_hal_async::spi::SpiDevice)
    static SPI_BUS: StaticCell<SharedSpiBus> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(hardware.spi_bus));

    // The stack needs to be static so that it can be used in tasks.
    STACK.init(hardware.sta_stack);

    // Init the vs1053 spi speeds
    let mut spi_sci_config = SpiConfig::default();
    spi_sci_config.frequency = 250.kHz();

    let mut spi_sdi_config = SpiConfig::default();
    spi_sdi_config.frequency = 8000.kHz();

    let spi_sci_device: SpiDeviceWithConfig<'_, NoopRawMutex, Spi<'_, esp_hal::Async>, Output<'_>> =
        SpiDeviceWithConfig::new(spi_bus, hardware.xcs, spi_sci_config);
    let spi_sdi_device = SpiDeviceWithConfig::new(spi_bus, hardware.xdcs, spi_sdi_config);

    let vs1053_driver: Vs1053DriverType = Vs1053Driver::new(
        spi_sci_device,
        spi_sdi_device,
        hardware.dreq,
        hardware.reset,
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
    //print_registers().await;

    // Setting up the network
    spawner.spawn(wifi_connect(hardware.wifi_controller)).ok();
    spawner.spawn(run_network_stack(hardware.runner)).ok();

    // Tasks to handle peripherals
    spawner.spawn(button_monitor(hardware.button_pin)).ok();
    spawner.spawn(wifi_connected_indicator(hardware.led)).ok();

    // Select station  TODO
    //static STATIONS: StaticCell<Stations> = StaticCell::new();

    // PROBLEM CODE
    // Load the stations.
    //TODO currently a test load
    let mut stations = Stations::new();
    esp_println::println!("DEBUG: About to load station");
    stations.add_station(STATION.0, STATION.1).unwrap();
    esp_println::println!("DEBUG: Station loaded");

    // if stations.load_stations().is_err() {
    //     esp_println::println!("ERROR: Cannot load stations!");
    // }
    // esp_println::println!("DEBUG: Loaded stations");

    // //let stations = STATIONS.init(stations);

    // let station_id = 0;
    // let station = stations
    //     .get_station(station_id)
    //     .expect("ERROR: Cannot get station {station_id}");

    // static CURRENT_STATION: StaticCell<Station> = StaticCell::new();
    // let current_station = CURRENT_STATION.init(station);

    esp_println::println!("DEBUG: Getting the current station");

    static CURRENT_STATION: StaticCell<Station> = StaticCell::new();
    let station = Station::new("SWR3", STATION_URL).unwrap();
    let current_station = CURRENT_STATION.init(station);
    esp_println::println!("DEBUG: Got  current station");

    // Streaming and playing music
    spawner
        .spawn(stream(hardware.sta_stack, current_station))
        .ok();

    spawner.spawn(play_music()).ok();
    esp_println::println!("DEBUG: All tasks spawned");
}

// async fn print_registers() {
//     let mut driver_unlocked = CODEC_DRIVER.lock().await;
//     if let Some(driver) = driver_unlocked.as_mut() {
//         // Set the volume so we can see the value when we dump the registers
//         let left_vol = 0x11;
//         let right_vol = 0x22;

//         driver.set_volume(left_vol, right_vol).await.unwrap();
//         // Should see 1122 as the vol reg
//         let registers = driver.dump_registers().await.unwrap();

//         esp_println::println!("Dumped registers:");
//         esp_println::println!("mode: {:X}", registers.mode);
//         esp_println::println!("status: {:X}", registers.status);
//         esp_println::println!("clockf: {:X}", registers.clock_f);
//         esp_println::println!("volume: {:X}", registers.volume);
//         esp_println::println!("audio_data : {:X}", registers.audio_data);
//     } else {
//         esp_println::println!("ERROR: Could not print registers");
//     }
// }
