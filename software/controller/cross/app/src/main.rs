#![no_main]
#![no_std]
// Make std available when testing
//#![cfg_attr(not(test), no_std)]

//! An internet radio
//!

// See this about having functions to setup the peripherals and avoid the borrow problem:
// https://users.rust-lang.org/t/how-to-borrow-peripherals-struct/83565/2

// [ ]  Tidy up the use statements

mod async_delay;
mod constants;
use constants::{NUMBER_SOCKETS_STACK_RESOURCES, NUMBER_SOCKETS_TCP_CLIENT_STATE};
mod initialized_peripherals;

use initialized_peripherals::InitilizedPeripherals;

use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, Output},
    spi::master::{Config as SpiConfig, Spi},
    time::RateExtU32,
};

use embassy_executor::Spawner;

use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    Runner, Stack,
};

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;

use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
    channel::Channel,
    mutex::Mutex,
    signal,
};

use embassy_time::{Duration, Timer};

use embedded_io_async::Read;

use esp_wifi::wifi::{
    AuthMethod, ClientConfiguration, Configuration, WifiController, WifiDevice, WifiStaDevice,
};

use reqwless::{client::HttpClient, request};

use static_cell::StaticCell;

use async_delay::AsyncDelay;
use core::str::from_utf8;

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

// Signal that the web should be accessed
static ACCESS_WEB_SIGNAL: signal::Signal<CriticalSectionRawMutex, bool> = signal::Signal::new();

// Channel to stream internet radio content to the mp3 codec
static CHANNEL: Channel<CriticalSectionRawMutex, [u8; 32], 64> = Channel::new();

// Some mp3 music for testing
static _TEST_MUSIC: &[u8; 55302] = include_bytes!("../../../resources/music-16b-2c-8000hz.mp3");

// Wifi secrets stored as environment varaibles
const SSID: &str = env!("WLAN-SSID");
const PASSWORD: &str = env!("WLAN-PASSWORD");

const DEBOUNCE_DURATION: u64 = 100; // Milliseconds  TODO use fugit?

#[embassy_executor::task]
async fn button_monitor(mut pin: Input<'static>) {
    loop {
        pin.wait_for_falling_edge().await;

        // Debounce
        // TODO see also https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/debounce.rs
        Timer::after(Duration::from_millis(DEBOUNCE_DURATION)).await;

        if pin.is_low() {
            // Pin is still low so acknowledge
            esp_println::println!("Button pressed after debounce!");

            // Now access the web by sending a signal
            ACCESS_WEB_SIGNAL.signal(true)
        }
    }
}

const BUFFER_SIZE: usize = 2560;

/// This task only accesses the web when  ACCESS_WEB_SIGNAL is signalled.
#[embassy_executor::task]
async fn access_web(stack: Stack<'static>) {
    let mut rx_buffer = [0; BUFFER_SIZE];

    loop {
        ACCESS_WEB_SIGNAL.wait().await;

        esp_println::println!("Access web task");

        loop {
            if stack.is_link_up() {
                break;
            }
            Timer::after(Duration::from_millis(500)).await;
        }

        let client_state =
            TcpClientState::<NUMBER_SOCKETS_TCP_CLIENT_STATE, BUFFER_SIZE, BUFFER_SIZE>::new();
        let tcp_client = TcpClient::new(stack, &client_state);
        let dns = DnsSocket::new(stack);
        let mut http_client = HttpClient::new(&tcp_client, &dns);

        esp_println::println!("Setting up request");

        let mut request = http_client
            .request(
                request::Method::GET,
                "http://andrew-doble.hier-im-netz.de/ir/stations.txt",
            )
            .await
            .unwrap();

        esp_println::println!("Sending request, reading response");
        let response = request.send(&mut rx_buffer).await.unwrap();

        esp_println::println!("Getting body");

        esp_println::println!("Http body:");

        // This approach can be used to read a stream
        let mut reader = response.body().reader();
        let mut small_buffer: [u8; 32] = [0; 32];

        loop {
            let res = reader.read(&mut small_buffer).await;
            match res {
                Ok(size) if size == 0 => {
                    esp_println::println!("EOF");
                    break;
                }
                Ok(size) if size > 0 => {
                    //let content = from_utf8(&small_buffer).unwrap();
                    //esp_println::print!("{content}");
                    CHANNEL.send(small_buffer).await;
                    continue;
                }
                Err(err) => {
                    esp_println::println!("Error in reading {:?}", err);
                    break;
                }
                _ => {
                    esp_println::println!("Unknown read condition");
                    break;
                }
            }
        }

        // This approach is better to read a page (for instance, the stations list)
        // let body = from_utf8(response.body().read_to_end().await.unwrap()).unwrap();
        // esp_println::println!("Http body:");
        // esp_println::println!("{body}");

        ACCESS_WEB_SIGNAL.reset();

        // TODO Is this delay required?
        //Timer::after(Duration::from_millis(3000)).await;
    }
}

#[embassy_executor::task]
async fn process_channel() {
    loop {
        let data = CHANNEL.receive().await;
        let content = from_utf8(&data).unwrap();
        esp_println::print!("{content}");
    }
}

#[embassy_executor::task]
async fn notification_task() {
    loop {
        esp_println::println!("Press button to access web page!");
        Timer::after(Duration::from_millis(3_000)).await;
    }
}

#[embassy_executor::task]
async fn wifi_connect(mut controller: WifiController<'static>) {
    esp_println::println!("Wait to get wifi connected");

    loop {
        if !matches!(controller.is_started(), Ok(true)) {
            let mut auth_method = AuthMethod::WPA2Personal;
            if PASSWORD.is_empty() {
                auth_method = AuthMethod::None;
            }

            let wifi_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                auth_method, // TODO: Is AuthMethod::WPA2Personal the default?
                ..Default::default()
            });
            let res = controller.set_configuration(&wifi_config);
            esp_println::println!("Wi-Fi set_configuration returned {:?}", res);

            esp_println::println!("Starting wifi");
            controller.start_async().await.unwrap();
            esp_println::println!("Wifi started!");
        }

        match controller.connect_async().await {
            Ok(_) => esp_println::println!("Wifi connected!"),
            Err(e) => {
                esp_println::println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

// Run the network stack.
// This must be called in a background task, to process network events.
#[embassy_executor::task]
async fn run_network_stack(mut runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) {
    runner.run().await
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::println!("Init!");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(72 * 1024); // TODO is this too big!

    // Initialise gpio ,spi and  wifi peripherals
    let init_peripherals =
        InitilizedPeripherals::init::<NUMBER_SOCKETS_STACK_RESOURCES>(peripherals);

    let delay = AsyncDelay::new();

    // This is the way to initialize esp hal embassy for the the esp32c3
    // according to the example
    // https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point_with_sta.rs
    esp_hal_embassy::init(init_peripherals.system_timer.alarm0);

    // Create the SPI from the HAL. This implements SpiBus, not SpiDevice!
    // Seems to only work with SPI2 - TODO is this true?
    let spi_bus: Spi<'_, esp_hal::Async> = Spi::new(init_peripherals.spi2, SpiConfig::default())
        .expect("Panic: Could not initialize SPI")
        .with_sck(init_peripherals.sclk)
        .with_mosi(init_peripherals.mosi)
        .with_miso(init_peripherals.miso)
        .into_async();

    static SPI_BUS: StaticCell<SharedSpiBus> = StaticCell::new();
    // Need to convert the spi driver into an static blocking async version so that if can be accepted
    // by vs1053_driver::Vs1052Driver (which takes embedded_hal_async::spi::SpiDevice)
    // let spi_bus_blocking = BlockingAsync::new(spi_bus);
    // let spi_bus = SPI_BUS.init(Mutex::new(spi_bus_blocking));
    let spi_bus = SPI_BUS.init(Mutex::new(spi_bus));

    // The stack needs to be static so that it can be used in tasks.
    STACK.init(init_peripherals.sta_stack);

    // Init the vs1053 spi speeds
    let mut spi_sci_config = SpiConfig::default();
    spi_sci_config.frequency = 250.kHz();

    let mut spi_sdi_config = SpiConfig::default();
    spi_sdi_config.frequency = 8000.kHz();

    let spi_sci_device = SpiDeviceWithConfig::new(spi_bus, init_peripherals.xcs, spi_sci_config);
    let spi_sdi_device = SpiDeviceWithConfig::new(spi_bus, init_peripherals.xdcs, spi_sdi_config);

    let mut vs1053_driver: Vs1053Driver<
        SpiDeviceWithConfig<'_, NoopRawMutex, Spi<'_, esp_hal::Async>, Output<'_>>,
        Input<'_>,
        Output<'_>,
        AsyncDelay,
    > = Vs1053Driver::new(
        spi_sci_device,
        spi_sdi_device,
        init_peripherals.dreq,
        init_peripherals.reset,
        delay,
    )
    .unwrap();

    vs1053_driver.begin().await.unwrap();

    print_registers(&mut vs1053_driver).await;

    spawner
        .spawn(wifi_connect(init_peripherals.wifi_controller))
        .ok();
    spawner
        .spawn(run_network_stack(init_peripherals.runner))
        .ok();
    spawner
        .spawn(button_monitor(init_peripherals.button_pin))
        .ok();
    spawner.spawn(notification_task()).ok();
    spawner.spawn(access_web(init_peripherals.sta_stack)).ok();
    spawner.spawn(process_channel()).ok();
}

async fn print_registers(driver: &mut Vs1053DriverType<'_>) {
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
}
