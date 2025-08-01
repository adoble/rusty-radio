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
use constants::{
    MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN, MULTIPLEXER_DEVICE_ADDR, NUMBER_PRESETS,
    NUMBER_SOCKETS_STACK_RESOURCES,
};

mod hardware;
use hardware::Hardware;

mod task;
use task::{
    play_music::play_music,
    station_indicator::station_indicator,
    //read_test_music::read_test_music,
    stream::stream,
    //stream2::stream2,
    sync::CODEC_DRIVER,
    sync::MULTIPLEXER_DRIVER,

    //access_radio_stations::access_radio_stations,
    tuner::tuner,
    //wifi_connected_indicator::wifi_connected_indicator,
    wifi_tasks::{run_network_stack, wifi_connect},
};

mod front_panel;
use front_panel::FrontPanel;

mod sendable_multiplexer_driver;
use sendable_multiplexer_driver::SendableMultiplexerDriver;

use stations::{Station, Stations};

// External crates
//use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, Output},
    spi::master::{Config as SpiConfig, Spi},
    //time::RateExtU32,
    time::Rate,
};

use embassy_executor::Spawner;

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

use static_cell::StaticCell;

use async_delay::AsyncDelay;

static_assertions::const_assert!(true);

use vs1053_driver::Vs1053Driver;

use mcp23s17_async::Mcp23s17;

static STACK: StaticCell<embassy_net::Stack> = StaticCell::new();

static FRONT_PANEL: StaticCell<FrontPanel> = StaticCell::new();

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

type RadioStation = Station<MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN>;
type RadioStations = Stations<MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN, NUMBER_PRESETS>;

static RADIO_STATIONS: StaticCell<RadioStations> = StaticCell::new();

//const MULTIPLEXER_DEVICE_ADDR: u8 = 0x00;

// INFO: Notes on stations
// NOTE: This station does a number of redirects by setting the response header "location". Note that it does
// not give a return code 3xx which is strange.
// Anaylsed with Google HAR analyser https://toolbox.googleapps.com/apps/har_analyzer/
// For a description of the location field see: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Location
//const STATION_URL: &str = "http://liveradio.swr.de/sw282p3/swr3/play.mp3";

// NOTE: This station doesn't seem to have redirects (as of now) so used to test the basic functionality
//const STATION_URL: &str = "http://listen.181fm.com/181-classical_128k.mp3";

// Local server for testing
//const STATION_URL: &str = "http://192.168.2.107:8080/music/2"; // Hijo de la Luna. 128 kb/s

//const STATION: (&str, &str) = ("SWR3", "http://liveradio.swr.de/sw282p3/swr3/play.mp3");

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::println!("INFO: Rusty Radio started");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // See this: https://github.com/esp-rs/esp-hal/blob/v0.21.1/esp-wifi/MIGRATING-0.9.md#memory-allocation
    // Size has been empirically determined.
    esp_alloc::heap_allocator!(size: 76 * 1024);

    //esp_alloc::heap_allocator!(48 * 1024);   //Recommanded

    // Initialise gpio ,spi and wifi peripherals. The initialised peripherals are then fields in the hardware struct
    // and are given symbolic names.
    let hardware = Hardware::init::<NUMBER_SOCKETS_STACK_RESOURCES>(peripherals);

    let delay = AsyncDelay::new();

    // The stack needs to be static so that it can be used in tasks.
    STACK.init(hardware.sta_stack);

    // This is the way to initialize esp hal embassy for the the esp32c3
    // according to the example
    // https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point_with_sta.rs
    esp_hal_embassy::init(hardware.system_timer.alarm0);

    // Need to convert the spi driver into an static blocking async version so that if can be accepted
    // by vs1053_driver::Vs1052Driver (which takes embedded_hal_async::spi::SpiDevice)
    static SPI_BUS: StaticCell<SharedSpiBus> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(hardware.spi_bus));

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
    // Setup spi for the front panel controller
    // spi_multiplexer_config.frequency = 10.MHz();
    let spi_multiplexer_config = SpiConfig::default().with_frequency(Rate::from_mhz(10));

    let spi_multiplexer_device: SpiDeviceWithConfig<
        '_,
        CriticalSectionRawMutex,
        Spi<'_, esp_hal::Async>,
        Output<'_>,
    > = SpiDeviceWithConfig::new(spi_bus, hardware.mux_cs, spi_multiplexer_config);

    // Set up the mutiplexer driver and provide a mutex for it. Using the sendable version.
    let multiplexer_driver: SendableMultiplexerDriver = SendableMultiplexerDriver(
        Mcp23s17::new(spi_multiplexer_device, MULTIPLEXER_DEVICE_ADDR)
            .await
            .unwrap(),
    );
    {
        *(MULTIPLEXER_DRIVER.lock().await) = Some(multiplexer_driver);
    }

    // Set up the mutiplexer driver and provide a mutex for it.
    // let multiplexer_driver: Mcp23s17<
    //     SpiDeviceWithConfig<'_, CriticalSectionRawMutex, Spi<'_, esp_hal::Async>, Output<'_>>,
    // > = Mcp23s17::new(spi_multiplexer_device, MULTIPLEXER_DEVICE_ADDR)
    //     .await
    //     .unwrap();
    // {
    //     *(MULTIPLEXER_DRIVER.lock().await) = Some(multiplexer_driver);
    // }

    let front_panel = FrontPanel::new()
        .await
        .expect("ERROR: Cannot initialise front panel");

    let front_panel = FRONT_PANEL.init(front_panel);

    // set up the stations

    let stations_data = include_bytes!("../../../resources/stations.txt");

    let stations = RadioStations::load(stations_data).expect("ERROR: Cannot load stations");
    let stations = RADIO_STATIONS.init(stations);

    // Print the registers using the shared driver for the vs1053
    //print_registers().await;

    // TODO set these to must_spawn

    // Setting up the network
    spawner.spawn(wifi_connect(hardware.wifi_controller)).ok();
    spawner.spawn(run_network_stack(hardware.runner)).ok();

    // Tasks to handle peripherals
    //spawner.spawn(tuner(hardware.button_pin)).ok();
    // spawner
    //     .spawn(tuner(stations, front_panel, hardware.intr))
    //     .ok();
    spawner
        .spawn(tuner(stations, front_panel, hardware.intr))
        .ok();
    //spawner.spawn(wifi_connected_indicator(hardware.led)).ok();

    // Streaming and playing music
    spawner.spawn(stream(hardware.sta_stack)).ok();

    spawner.spawn(play_music()).ok();

    // Showing on the panel LED when a station has been tuned in
    // TODO This is a temporary solution until the display is ready.
    spawner.spawn(station_indicator(front_panel)).ok();
    // spawner
    //     .spawn(station_indicator(spi_bus, hardware.mux_cs))
    //     .ok();
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
