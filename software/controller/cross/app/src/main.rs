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
    play_music::play_music,
    //read_test_music::read_test_music,
    stream::stream,
    //stream2::stream2,
    sync::CODEC_DRIVER,
    sync::MULTIPLEXER_DRIVER,

    //access_radio_stations::access_radio_stations,
    tuner2::tuner2,
    //wifi_connected_indicator::wifi_connected_indicator,
    wifi_tasks::{run_network_stack, wifi_connect},
};

mod front_panel;
use front_panel::FrontPanel;

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

use static_cell::StaticCell;

use async_delay::AsyncDelay;

static_assertions::const_assert!(true);

use vs1053_driver::Vs1053Driver;

use mcp23s17_async::Mcp23s17;

static STACK: StaticCell<embassy_net::Stack> = StaticCell::new();

static FRONT_PANEL: StaticCell<FrontPanel> = StaticCell::new();

type SharedSpiBus = Mutex<NoopRawMutex, Spi<'static, esp_hal::Async>>;

type Vs1053DriverType<'a> = Vs1053Driver<
    SpiDeviceWithConfig<'a, NoopRawMutex, Spi<'a, esp_hal::Async>, Output<'a>>,
    Input<'a>,
    Output<'a>,
    AsyncDelay,
>;

type MultiplexerDriverType<'a> =
    Mcp23s17<SpiDeviceWithConfig<'a, NoopRawMutex, Spi<'a, esp_hal::Async>, Output<'a>>>;

const MULTIPLEXER_DEVICE_ADDR: u8 = 0x00;

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
    let hardware = Hardware::init::<NUMBER_SOCKETS_STACK_RESOURCES>(peripherals);

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
    // TODO move the somewhere else in the code
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
    // Setup spi for the front panel controller
    let mut spi_multiplexer_config = SpiConfig::default();
    spi_multiplexer_config.frequency = 10.MHz();

    let spi_multiplexer_device: SpiDeviceWithConfig<
        '_,
        NoopRawMutex,
        Spi<'_, esp_hal::Async>,
        Output<'_>,
    > = SpiDeviceWithConfig::new(spi_bus, hardware.mux_cs, spi_multiplexer_config);

    // TODO the new function is not protected with a mutex. Do we need a begin() function? Can I just do this with  a new function?
    // Do I even need a mutex here as this is not beinh done in a seperate task.
    let multiplexer_driver: Mcp23s17<
        SpiDeviceWithConfig<'_, NoopRawMutex, Spi<'_, esp_hal::Async>, Output<'_>>,
    > = Mcp23s17::new(spi_multiplexer_device, MULTIPLEXER_DEVICE_ADDR)
        .await
        .unwrap();

    // Setup the mutex for the mutiplexer driver
    {
        *(MULTIPLEXER_DRIVER.lock().await) = Some(multiplexer_driver);
    }

    let front_panel = FrontPanel::new()
        .await
        .expect("ERROR: Cannot initialise front panel");

    let front_panel = FRONT_PANEL.init(front_panel);

    // Print the registers using the shared driver for the vs1053
    //print_registers().await;

    // TODO set these to must_spawn

    // Setting up the network
    spawner.spawn(wifi_connect(hardware.wifi_controller)).ok();
    spawner.spawn(run_network_stack(hardware.runner)).ok();

    // Tasks to handle peripherals
    //spawner.spawn(tuner(hardware.button_pin)).ok();
    spawner.spawn(tuner2(front_panel, hardware.intr)).ok();
    //spawner.spawn(wifi_connected_indicator(hardware.led)).ok();

    // Streaming and playing music
    spawner.spawn(stream(hardware.sta_stack)).ok();

    spawner.spawn(play_music()).ok();

    // spawner
    //     .spawn(test_button_board(front_panel, &hardware.intr))
    //     .ok();
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
