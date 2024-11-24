//! An internet radio

#![no_std]
#![no_main]

use core::str::from_utf8;

use embassy_executor::Spawner;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
//use esp_hal::gpio::{AnyPin, Input, Io, Level, Output, Pull};
use esp_hal::gpio::{AnyPin, Input, Io, Pull};
use esp_hal::timer::timg::TimerGroup;

use esp_hal::{prelude::*, rng::Rng};

use embedded_io_async::Read;
use esp_wifi::wifi::{WifiController, WifiDevice};
use esp_wifi::{
    init,
    wifi::{AuthMethod, ClientConfiguration, Configuration, WifiStaDevice},
    EspWifiInitFor,
};
use reqwless::client::HttpClient;
use reqwless::request;
use static_cell::StaticCell;

use static_assertions::{self, const_assert};

static_assertions::const_assert!(true);

const NUMBER_SOCKETS_STACK_RESOURCES: usize = 3;
const NUMBER_SOCKETS_TCP_CLIENT_STATE: usize = 3;

// The number of sockets specified for StackResources needs to be the same or higher then the number of sockets specified
// in setting up the TcpClientState. Getting this wrong resukts in the program crashing - and took me a long time
// to figure out the cause.
// This is checked at compilation time by this macro.
// An alterantive would be to use the same constant for setting up both StackResources and TcpClientState
const_assert!(NUMBER_SOCKETS_STACK_RESOURCES >= NUMBER_SOCKETS_TCP_CLIENT_STATE);

//const NUMBER_SOCKETS: usize = 3; // Used by more than one package and needs to be in sync

static RESOURCES: StaticCell<embassy_net::StackResources<NUMBER_SOCKETS_STACK_RESOURCES>> =
    StaticCell::new();
static STACK: StaticCell<embassy_net::Stack<WifiDevice<WifiStaDevice>>> = StaticCell::new();

// Signal that the web should be accessed
static ACCESS_WEB_SIGNAL: signal::Signal<CriticalSectionRawMutex, bool> = signal::Signal::new();

static CHANNEL: Channel<CriticalSectionRawMutex, [u8; 32], 64> = Channel::new();

static TEST_MUSIC: &[u8; 55302] = include_bytes!("resources/music-16b-2c-8000hz.mp3");

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
            controller.start().await.unwrap();
            esp_println::println!("Wifi started!");
        }

        match controller.connect().await {
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

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let button_pin = Input::new(io.pins.gpio1, Pull::Up);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timg1 = TimerGroup::new(peripherals.TIMG1);

    // Initialize the timers used for Wifi
    // TODO: can the embassy timers be used?
    //let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let init = init(
        EspWifiInitFor::Wifi,
        timg1.timer0,
        Rng::new(&mut peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let (wifi_device, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

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

    spawner.spawn(wifi_connect(controller)).ok();
    spawner.spawn(run_network_stack(stack)).ok();
    spawner.spawn(button_monitor(button_pin)).ok();
    spawner.spawn(notification_task()).ok();
    spawner.spawn(access_web(stack)).ok();
    spawner.spawn(process_channel()).ok();
}
