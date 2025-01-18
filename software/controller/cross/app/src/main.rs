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

use esp_hal::time::RateExtU32;

use core::str::from_utf8;

use embassy_executor::Spawner;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::Runner;
use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::{Input, Level, Output, Pull};
use esp_hal::spi::master::{Config as SpiConfig, Spi};

use esp_hal::timer::systimer::SystemTimer;
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};

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

const NUMBER_SOCKETS_STACK_RESOURCES: usize = 3;
const NUMBER_SOCKETS_TCP_CLIENT_STATE: usize = 3;

// The number of sockets specified for StackResources needs to be the same or higher then the number of sockets specified
// in setting up the TcpClientState. Getting this wrong resukts in the program crashing - and took me a long time
// to figure out the cause.
// This is checked at compilation time by this macro.
// An alternative would be to use the same constant for setting up both StackResources and TcpClientState
const_assert!(NUMBER_SOCKETS_STACK_RESOURCES >= NUMBER_SOCKETS_TCP_CLIENT_STATE);

static ESP_WIFI_CONTROLLER: StaticCell<EspWifiController<'static>> = StaticCell::new();

static RESOURCES: StaticCell<embassy_net::StackResources<NUMBER_SOCKETS_STACK_RESOURCES>> =
    StaticCell::new();
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

    let button_pin = Input::new(peripherals.GPIO1, Pull::Up);

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
    // Seems to only work with SPI2 - TODO is this true?
    let spi_bus: Spi<'_, esp_hal::Async> = Spi::new(peripherals.SPI2, SpiConfig::default())
        .expect("Panic: Could not initialize SPI")
        .with_sck(sclk)
        .with_mosi(mosi)
        .with_miso(miso)
        .into_async();

    static SPI_BUS: StaticCell<SharedSpiBus> = StaticCell::new();
    // Need to convert the spi driver into an blocking async version so that if can be accepted
    // by vs1053_driver::Vs1052Driver (which takes embedded_hal_async::spi::SpiDevice)
    // let spi_bus_blocking = BlockingAsync::new(spi_bus);
    // let spi_bus = SPI_BUS.init(Mutex::new(spi_bus_blocking));
    let spi_bus = SPI_BUS.init(Mutex::new(spi_bus));

    let mut esp32_rng = Rng::new(peripherals.RNG);

    let init = ESP_WIFI_CONTROLLER.uninit().write(
        init(
            timg1.timer0,
            //Rng::new(peripherals.RNG.clone()),
            esp32_rng.clone(),
            peripherals.RADIO_CLK,
        )
        .unwrap(),
    );

    let wifi = peripherals.WIFI;
    let (wifi_device, controller) =
        esp_wifi::wifi::new_with_mode(init, wifi, WifiStaDevice).unwrap();

    // This is the way to initialize esp hal embassy for the the esp32c3
    // according to the example
    // https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point_with_sta.rs
    let systimer = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(systimer.alarm0);

    let sta_config = embassy_net::Config::dhcpv4(Default::default());

    // Random seed.
    // Taken from example line 104 https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point_with_sta.rs
    let seed = (esp32_rng.random() as u64) << 32 | esp32_rng.random() as u64;

    // Init network stacks
    let (sta_stack, sta_runner) = embassy_net::new(
        wifi_device,
        sta_config,
        RESOURCES.init(embassy_net::StackResources::new()), // mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    // The stack needs to be static so that it can be used in tasks.
    STACK.init(sta_stack);

    // Init the vs1053 spi speeds
    let mut spi_sci_config = SpiConfig::default();
    spi_sci_config.frequency = 250.kHz();

    let mut spi_sdi_config = SpiConfig::default();
    spi_sdi_config.frequency = 8000.kHz();

    let spi_sci_device = SpiDeviceWithConfig::new(spi_bus, xcs, spi_sci_config);
    let spi_sdi_device = SpiDeviceWithConfig::new(spi_bus, xdcs, spi_sdi_config);

    // This was a test to see if issue esp-hal #2885 has been corrected. It has!
    // use embedded_hal_async::spi::Operation;
    // spi_sci_device
    //     .transaction(&mut [Operation::Write(&[0x00, 0x00])]) // --> AAA. But transaction is not defined altough SpiDeviceWithConfig shoudl implement  embeddded_hal_async::spi::SpiDevice
    //     .await
    //     .unwrap();

    let mut vs1053_driver: Vs1053Driver<
        SpiDeviceWithConfig<'_, NoopRawMutex, Spi<'_, esp_hal::Async>, Output<'_>>,
        Input<'_>,
        Output<'_>,
        AsyncDelay,
    > = Vs1053Driver::new(spi_sci_device, spi_sdi_device, dreq, reset, delay).unwrap();

    vs1053_driver.begin().await.unwrap();

    print_registers(&mut vs1053_driver).await;

    spawner.spawn(wifi_connect(controller)).ok();
    spawner.spawn(run_network_stack(sta_runner)).ok();
    spawner.spawn(button_monitor(button_pin)).ok();
    spawner.spawn(notification_task()).ok();
    spawner.spawn(access_web(sta_stack)).ok();
    spawner.spawn(process_channel()).ok();
}

async fn print_registers(driver: &mut Vs1053DriverType<'_>) {
    //driver.begin().await.unwrap();

    // Set the volume so we can see the value when we dump the registers
    let left_vol = 0x11;
    let right_vol = 0x22;
    driver.set_volume(left_vol, right_vol).await.unwrap();
    // Should see 1122 as the vol reg
    let registers = driver.dump_registers().await.unwrap();

    esp_println::println!("Dump registers after begin():");
    esp_println::println!("mode: {:X}", registers.mode);
    esp_println::println!("status: {:X}", registers.status);
    esp_println::println!("clockf: {:X}", registers.clock_f);
    esp_println::println!("volume: {:X}", registers.volume);
    esp_println::println!("audio_data : {:X}", registers.audio_data);
}
