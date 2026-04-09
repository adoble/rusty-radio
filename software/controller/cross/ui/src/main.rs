#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
// TODO see if we can reinstate this
//#![deny(clippy::large_stack_frames)]

mod hardware;
use hardware::Hardware;

mod front_panel;
use front_panel::FrontPanel;

mod sendable_multiplexer_driver;
use sendable_multiplexer_driver::SendableMultiplexerDriver;

mod task;
use task::tuner::tuner;

mod station_config;
pub use station_config::StationConfig;

use mcp23s17_async::Mcp23s17;

static STATION_CONFIG: StaticCell<StationConfig> = StaticCell::new();

// Drivers
static FRONT_PANEL: StaticCell<FrontPanel> = StaticCell::new();

type SharedSpiBus = Mutex<CriticalSectionRawMutex, Spi<'static, esp_hal::Async>>;

pub type RadioStationId = usize;

pub type MultiplexerDriverType<'a> =
    Mcp23s17<SpiDeviceWithConfig<'a, CriticalSectionRawMutex, Spi<'a, esp_hal::Async>, Output<'a>>>;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use esp_hal::{
    clock::CpuClock,
    gpio::{Input, Output},
    ram,
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use static_cell::StaticCell;

// The address of the mcp23s17 device. This is hardwared on the front panel.
pub const MULTIPLEXER_DEVICE_ADDR: u8 = 0x00;

// We need to share the front panel driver between tasks so put it in a static mutex
pub static MULTIPLEXER_DRIVER: Mutex<CriticalSectionRawMutex, Option<SendableMultiplexerDriver>> =
    Mutex::new(None);

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) {
    // generator version: 1.2.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Initialise gpio ,spi and wifi peripherals. The initialised peripherals are then fields in the hardware struct
    // and are given symbolic names.
    let hardware = Hardware::init(peripherals);

    esp_rtos::start(hardware.system_timer.alarm0, hardware.software_interrupt0);

    // Wrap the spi_bus in a Mutex and store statically
    static SPI_BUS: StaticCell<SharedSpiBus> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(hardware.spi_bus_ui));

    // Setup spi for the front panel controller
    // Altough the mutiplexer SPI speed can go up to 10MHz, using a lower frequency works just fine
    // (and gives less problems with transmission lines effects on the breadboard).
    let spi_multiplexer_config = SpiConfig::default().with_frequency(Rate::from_mhz(1));

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

    // Initalise the front panel containing the controls and make it static
    let front_panel = FrontPanel::new()
        .await
        .expect("ERROR: Cannot initialise front panel");

    let front_panel = FRONT_PANEL.init(front_panel);

    // Get the stations configuration from the radio processor and make it static
    let station_config = get_station_config().await;
    let station_config = STATION_CONFIG.init(station_config);

    // Spawn the tuner task to read in the front panel controls and
    // convert this information to station ids.
    spawner.must_spawn(tuner(station_config, front_panel));

    // loop {
    //     Timer::after(Duration::from_secs(1)).await;
    //     esp_println::println!("INFO: Blink");
    // }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}

#[deprecated(note = "Replace with read from radio processor")]
async fn get_station_config() -> StationConfig {
    StationConfig {
        number_stations: 21,
        presets: Some([2, 3, 5, 7]),
    }
}
