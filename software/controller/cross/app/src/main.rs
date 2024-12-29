#![no_main]
#![no_std]
// Make std available when testing
//#![cfg_attr(not(test), no_std)]

//! An internet radio
//!

// See this about having functions to setup the peripherals and avoid the borrow problem:
// https://users.rust-lang.org/t/how-to-borrow-peripherals-struct/83565/2

mod async_delay;

use core::str::from_utf8;

//use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
//use embassy_sync::blocking_mutex::CriticalSectionMutex;
//use embassy_sync::blocking_mutex;
//use embassy_embedded_hal::adapter::YieldingAsync;
//use embassy_embedded_hal::shared_bus::asynch::spi::{SpiDevice, SpiDeviceWithConfig};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal;
use embassy_time::{Duration, Timer};
//use embedded_hal_async::spi::SpiDevice;
use esp_backtrace as _;
//use esp_hal::gpio::{AnyPin, Input, Io, Level, Output, Pull};
use esp_hal::gpio::{AnyPin, Input, Level, Output, Pull};
//use esp_hal::peripherals::Peripherals;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::spi::SpiMode;
use esp_hal::timer::timg::TimerGroup;

use esp_hal::{prelude::*, rng::Rng};

use embedded_io_async::Read;
use esp_wifi::wifi::{WifiController, WifiDevice};
use esp_wifi::{
    init,
    wifi::{AuthMethod, ClientConfiguration, Configuration, WifiStaDevice},
    //EspWifiInitFor,
    EspWifiController,
};
use reqwless::client::HttpClient;
use reqwless::request;
use static_cell::StaticCell;

use static_assertions::{self, const_assert};

use async_delay::AsyncDelay;

static_assertions::const_assert!(true);

use vs1053_driver::Vs1053Driver;

//use vs1053_driver::Vs1053Driver;

const NUMBER_SOCKETS_STACK_RESOURCES: usize = 3;
const NUMBER_SOCKETS_TCP_CLIENT_STATE: usize = 3;

// The number of sockets specified for StackResources needs to be the same or higher then the number of sockets specified
// in setting up the TcpClientState. Getting this wrong resukts in the program crashing - and took me a long time
// to figure out the cause.
// This is checked at compilation time by this macro.
// An alterantive would be to use the same constant for setting up both StackResources and TcpClientState
const_assert!(NUMBER_SOCKETS_STACK_RESOURCES >= NUMBER_SOCKETS_TCP_CLIENT_STATE);

//const NUMBER_SOCKETS: usize = 3; // Used by more than one package and needs to be in sync

static ESP_WIFI_CONTROLLER: StaticCell<EspWifiController<'static>> = StaticCell::new();

static RESOURCES: StaticCell<embassy_net::StackResources<NUMBER_SOCKETS_STACK_RESOURCES>> =
    StaticCell::new();
static STACK: StaticCell<embassy_net::Stack<WifiDevice<WifiStaDevice>>> = StaticCell::new();

//type SharedSpiBus = Mutex<NoopRawMutex, BlockingAsync<Spi<'static, esp_hal::Async>>>;
type SharedSpiBus = Mutex<NoopRawMutex, Spi<'static, esp_hal::Async>>;

// Signal that the web should be accessed
static ACCESS_WEB_SIGNAL: signal::Signal<CriticalSectionRawMutex, bool> = signal::Signal::new();

static CHANNEL: Channel<CriticalSectionRawMutex, [u8; 32], 64> = Channel::new();

static _TEST_MUSIC: &[u8; 55302] = include_bytes!("../../../resources/music-16b-2c-8000hz.mp3");

const SSID: &str = env!("WLAN-SSID");
const PASSWORD: &str = env!("WLAN-PASSWORD");

const DEBOUNCE_DURATION: u64 = 100; // Milliseconds  TODO use fugit?

#[embassy_executor::task]
async fn button_monitor(mut pin: Input<'static, AnyPin>) {
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

// const BUFFER_SIZE: usize = 8192;
const BUFFER_SIZE: usize = 2560;

/// This task only accesses the web when  ACCESS_WEB_SIGNAL is signalled.
#[embassy_executor::task]
async fn access_web(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
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
        let dns = DnsSocket::new(&stack);
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

        // ???
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

// #[embassy_executor::task]
// async fn dump_registers(
//     spi_bus: &'static SharedSpiBus,
//     xcs: Output<'static>,
//     xdcs: Output<'static>,
//     dreq: Input<'static>,
//     reset: Output<'static>,
//     delay: AsyncDelay,
// ) {
//     let spi_sci_device = SpiDevice::new(spi_bus, xcs);
//     let spi_sdi_device = SpiDevice::new(spi_bus, xdcs);

//     let mut driver = Vs1053Driver::new(spi_sci_device, spi_sdi_device, dreq, reset, delay).unwrap();

//     // Set the volume so we can see the value when we dump the registers
//     // let left_vol = 0x11;
//     // let right_vol = 0x22;
//     // driver.set_volume(left_vol, right_vol).await.unwrap();
//     // Should see 1122 as the vol reg

//     // Put this in a loop so that we can see it on the 'scope
//     loop {
//         let regs = driver.dump_registers().await.unwrap();

//         esp_println::println!("Dump registers:");
//         esp_println::println!("mode: {:X}", regs.mode);
//         esp_println::println!("status: {:X}", regs.status);
//         esp_println::println!("clockf: {:X}", regs.clock_f);
//         esp_println::println!("volume: {:X}", regs.volume);

//         Timer::after(Duration::from_millis(3000)).await;
//     }
// }

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
                auth_method,          // TODO: Is AuthMethod::WPA2Personal the default?
                ..Default::default()  // ANCHOR: client_config_end
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
async fn run_network_stack(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    //esp_println::logger::init_logger_from_env();
    esp_println::println!("Init!");

    // let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    esp_alloc::heap_allocator!(72 * 1024); // TODO is this too big!

    let button_pin = Input::new(peripherals.GPIO1, Pull::Up);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timg1 = TimerGroup::new(peripherals.TIMG1);

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

    // let spi_bus: Spi<'_, esp_hal::Async> = Spi::new_with_config(
    //     peripherals.SPI2,
    //     Config {
    //         frequency: 250.kHz(),
    //         mode: SpiMode::Mode0,
    //         ..Config::default()
    //     },
    // )
    // .with_sck(sclk)
    // .with_mosi(mosi)
    // .with_miso(miso)
    // .into_async();

    let spi_bus: Spi<'_, esp_hal::Async> = Spi::new(peripherals.SPI2)
        .with_sck(sclk)
        .with_mosi(mosi)
        .with_miso(miso)
        .into_async();

    static SPI_BUS: StaticCell<SharedSpiBus> = StaticCell::new();
    // Need to convert the spi driver into an blocking async version so that if can be accepted
    // by vs1053_driver::Vs1052Driver (which takes embedded_hal_async::spi::SpiDevice)
    //let yielding_spi_bus = YieldingAsync::new(spi_bus);
    //let spi_bus = SPI_BUS.init(Mutex::new(yielding_spi_bus));
    let spi_bus = SPI_BUS.init(Mutex::new(spi_bus));

    // Initialize the timers used for Wifi
    // TODO: can the embassy timers be used?
    //let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);

    //static ESP_WIFI_CONTROLLER: StaticCell<EspWifiController<'static>> = StaticCell::new();
    let init = ESP_WIFI_CONTROLLER.uninit().write(
        init(
            //EspWifiInitFor::Wifi,
            timg1.timer0,
            Rng::new(&mut peripherals.RNG),
            peripherals.RADIO_CLK,
        )
        .unwrap(),
    );

    // let init = init(
    //     //EspWifiInitFor::Wifi,
    //     timg1.timer0,
    //     Rng::new(&mut peripherals.RNG),
    //     peripherals.RADIO_CLK,
    // )
    // .unwrap();

    let wifi = peripherals.WIFI;
    let (wifi_device, controller) =
        esp_wifi::wifi::new_with_mode(init, wifi, WifiStaDevice).unwrap();

    // Init network stack

    let config = embassy_net::Config::dhcpv4(Default::default());

    let mut esp32_rng = Rng::new(&mut peripherals.RNG);

    //let seed = 1234; // very random, very secure seed  TODO use  esp_hal::rng::Rng
    let seed: u64 = esp32_rng.random().into();

    let stack = &*STACK.init(embassy_net::Stack::new(
        wifi_device,
        config,
        RESOURCES.init(embassy_net::StackResources::new()),
        seed,
    ));

    esp_hal_embassy::init(timg0.timer0);

    // Init the vs1053 spi speeds
    let spi_sci_config = Config {
        frequency: 250.kHz(),
        ..Default::default()
    };
    let spi_sdi_config = Config {
        frequency: 8000.kHz(),
        ..Default::default()
    };

    let spi_sci_device = SpiDeviceWithConfig::new(spi_bus, xcs, spi_sci_config);
    let spi_sdi_device = SpiDeviceWithConfig::new(spi_bus, xdcs, spi_sdi_config);

    // How to convert between  between embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig and embeddded_hal_async::spi::SpiDevice

    // let spi_sci_device_blocking = YieldingAsync::new(spi_sci_device);
    // let spi_sdi_device_blocking = YieldingAsync::new(spi_sdi_device);
    use embedded_hal_async::spi::Operation;

    spi_sci_device
        .transaction(&mut [Operation::Write(&[0x00, 0x00])])
        .await
        .unwrap();

    let mut vs1053_driver =
        Vs1053Driver::new(spi_sci_device, spi_sdi_device, dreq, reset, delay).unwrap();

    vs1053_driver.begin().await.unwrap();

    let registers = vs1053_driver.dump_registers().await.unwrap();

    esp_println::println!("Dump registers after begin():");
    esp_println::println!("mode: {:X}", registers.mode);
    esp_println::println!("status: {:X}", registers.status);
    esp_println::println!("clockf: {:X}", registers.clock_f);
    esp_println::println!("volume: {:X}", registers.volume);
    esp_println::println!("audio_data : {:X}", registers.audio_data);

    spawner.spawn(wifi_connect(controller)).ok();
    spawner.spawn(run_network_stack(stack)).ok();
    spawner.spawn(button_monitor(button_pin)).ok();
    spawner.spawn(notification_task()).ok();
    spawner.spawn(access_web(stack)).ok();
    spawner.spawn(process_channel()).ok();
    // spawner
    //     .spawn(dump_registers(spi_bus, xcs, xdcs, dreq, reset, delay))
    //     .ok();
}
