#![no_main]
#![no_std]

// Simple test of the VS1053 driver to generate signals on the spi pins.
// Based on this example : https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/shared_bus.rs
// TODO Use https://docs.embassy.dev/embassy-embedded-hal/git/default/shared_bus/asynch/spi/struct.SpiDeviceWithConfig.html
//      to change the speed of the spi devices

use embassy_embedded_hal::adapter::BlockingAsync;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;

use embassy_executor::Spawner;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;

use embassy_time::{Duration, Timer};

//use embedded_hal_bus::spi::CriticalSectionDevice;
use esp_backtrace as _;

use esp_hal::gpio::{Input, Level, Output, Pull};
use esp_hal::prelude::*;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::spi::SpiMode;
use esp_hal::timer::timg::TimerGroup;

use static_cell::StaticCell;
use vs1053_driver::Vs1053Driver;

// type SpiBus = Mutex<CriticalSectionRawMutex, Spi<'static, esp_hal::Blocking>>;
// type SpiBus = Mutex<NoopRawMutex, Spi<'static, esp_hal::Blocking>>;

// type SharedSpiBus = Mutex<NoopRawMutex, Spi<'static, esp_hal::Async>>;
type SharedSpiBus = Mutex<NoopRawMutex, BlockingAsync<Spi<'static, esp_hal::Async>>>;
use async_delay::AsyncDelay;
mod async_delay;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::println!("Starting test.");
    esp_println::println!("Signals should be seen on the SPI pins.");

    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    esp_alloc::heap_allocator!(72 * 1024); // TODO is this too big! Do we need it at all?

    //let button_pin = Input::new(peripherals.GPIO1, Pull::Up);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);
    let sclk = Output::new(peripherals.GPIO5, Level::Low);
    let mosi = Output::new(peripherals.GPIO6, Level::Low);
    let miso = Output::new(peripherals.GPIO7, Level::Low);
    let xcs = Output::new(peripherals.GPIO9, Level::Low);
    let xdcs = Output::new(peripherals.GPIO10, Level::Low);

    let dreq = Input::new(peripherals.GPIO8, Pull::None);
    let reset = Output::new(peripherals.GPIO20, Level::High);

    let delay = AsyncDelay::new();

    // Create the SPI from the HAL. This implements SpiBus, not SpiDevice!
    // Seems to only work with SPI2
    // let spi_sci = esp_hal::spi::master::Spi::new(peripherals.SPI2, 250.kHz(), SpiMode::Mode0);

    let spi_bus: Spi<'_, esp_hal::Async> = Spi::new_with_config(
        peripherals.SPI2,
        Config {
            frequency: 250.kHz(),
            mode: SpiMode::Mode0,
            ..Config::default()
        },
    )
    .with_sck(sclk)
    .with_mosi(mosi)
    .with_miso(miso)
    .into_async();

    static SPI_BUS: StaticCell<SharedSpiBus> = StaticCell::new();
    // Need to convert the spi driver into an blocking async version so that if can be accepted
    // by vs1053_driver::Vs1052Driver (which takes embedded_hal_async::spi::SpiDevice)
    let blocking_spi_bus = BlockingAsync::new(spi_bus);
    let spi_bus = SPI_BUS.init(Mutex::new(blocking_spi_bus));

    // Combine references to the SPI bus with a CS pin to get a SpiDevice for one device on the bus.
    // type SpiAsyncMutex =
    //     mutex::Mutex<CriticalSectionRawMutex, Spi<'static, esp_hal::peripherals::SPI2>>;

    // static SPI: StaticCell<SpiAsyncMutex> = StaticCell::new();
    //et spi_bus = SPI.init(mutex::Mutex::new(spi));

    spawner.must_spawn(sci_interface(spi_bus, xcs, xdcs, dreq, reset, delay));
}

#[embassy_executor::task]
async fn sci_interface(
    spi_bus: &'static SharedSpiBus,
    xcs: Output<'static>,
    xdcs: Output<'static>,
    dreq: Input<'static>,
    reset: Output<'static>,
    delay: AsyncDelay,
) {
    let spi_sci_device = SpiDevice::new(spi_bus, xcs);
    let spi_sdi_device = SpiDevice::new(spi_bus, xdcs);

    // let spi_sci_device = CriticalSectionDevice::new(spi_bus, xcs, delay).unwrap();
    // let spi_sdi_device = CriticalSectionDevice::new(spi_bus, xdcs, delay).unwrap();

    let mut driver = Vs1053Driver::new(spi_sci_device, spi_sdi_device, dreq, reset, delay).unwrap();

    // Put this in a loop so that we can see it on the 'scope
    loop {
        driver.sci_write(0x01, 0x19).await.unwrap();

        Timer::after(Duration::from_micros(300)).await;
    }
}
