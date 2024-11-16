//! embassy hello world
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

//% CHIPS: esp32 esp32c2 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3
//% FEATURES: embassy esp-hal-embassy/integrated-timers

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::{AnyPin, Input, Io, Level, Output, Pull};
use esp_hal::timer::timg::TimerGroup;
// use esp_hal::{
//     prelude::*,
//     rng::Rng,
//     time::{self, Duration},
// };
use esp_hal::{prelude::*, rng::Rng};

use esp_wifi::wifi::{WifiController, WifiDevice};
use esp_wifi::{
    init,
    wifi::{AuthMethod, ClientConfiguration, Configuration, WifiStaDevice},
    EspWifiInitFor,
};
use reqwless::client::HttpClient;
use static_cell::StaticCell;

static RESOURCES: StaticCell<embassy_net::StackResources<2>> = StaticCell::new();
static STACK: StaticCell<embassy_net::Stack<WifiDevice<WifiStaDevice>>> = StaticCell::new();

/// Signal that the web shoudl be accessed
/// TODO Maybe replace bool with something more meaningful
static ACCESS_WEB_SIGNAL: signal::Signal<CriticalSectionRawMutex, bool> = signal::Signal::new();

const SSID: &str = env!("WLAN-SSID");
const PASSWORD: &str = env!("WLAN-PASSWORD");

const DEBOUNCE_DURATION: u64 = 100; // Milliseconds  TODO use fugit

#[embassy_executor::task]
async fn run() {
    loop {
        esp_println::println!("Hello world from embassy using esp-hal-async!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}
// Blink something
#[embassy_executor::task]
async fn toggle_pin(mut pin: Output<'static, AnyPin>) {
    loop {
        pin.toggle();
        //esp_println::println!("Hello world from embassy using esp-hal-async!");
        Timer::after(Duration::from_millis(50)).await;
    }
}

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

const BUFFER_SIZE: usize = 8192;

/// This task only accesses the web when  ACCESS_WEB_SIGNAL is signalled.
#[embassy_executor::task]
async fn access_web(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    let mut rx_buffer = [0; 8192];
    let mut tx_buffer = [0; 8192];

    loop {
        ACCESS_WEB_SIGNAL.wait().await;

        esp_println::println!("Access web task");

        loop {
            if stack.is_link_up() {
                break;
            }
            Timer::after(Duration::from_millis(500)).await;
        }
        esp_println::println!("Waiting to get IP address...");
        loop {
            if let Some(config) = stack.config_v4() {
                esp_println::println!("Got IP: {}", config.address);
                break;
            }
            Timer::after(Duration::from_millis(500)).await;
        }

        let client_state = TcpClientState::<1, BUFFER_SIZE, BUFFER_SIZE>::new();
        let tcp_client = TcpClient::new(stack, &client_state);
        let dns = DnsSocket::new(&stack);
        let mut http_client = HttpClient::new(&tcp_client, &dns);

        ACCESS_WEB_SIGNAL.reset();
    }
}

#[embassy_executor::task]
async fn bing() {
    loop {
        esp_println::println!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
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
    // loop {
    //     let res = controller.is_connected();
    //     match res {
    //         Ok(connected) => {
    //             if connected {
    //                 esp_println::println!("Wifi {} is connected", SSID);
    //                 break;
    //             }
    //         }
    //         Err(err) => {
    //             esp_println::println!("{:?}", err);
    //             loop {}
    //         }
    //     }
    // }
}

// Run the network stack.
// This must be called in a background task, to process network events.
#[embassy_executor::task]
async fn run_network_stack(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}

// THIS DOES NOT WORK!
// But as it is not required leaving it here for the moment
// TODO Get wifi_scan to work as an async function or delete it.
// #[embassy_executor::task]
// async fn wifi_scan(controller: &'static mut WifiController<'static>) {
//     esp_println::println!("Start WiFi Scan");
//     let res: Result<(heapless::Vec<AccessPointInfo, 10>, usize), WifiError> = controller.scan_n();
//     esp_println::println!("Scan result:{:?}", res); // <------ Err

//     if let Ok((res, _count)) = res {
//         for ap in res {
//             //esp_println::println!("AP:{:?}", ap);
//             esp_println::println!("AP SSID {}, CHANNEL {}", ap.ssid, ap.channel);
//         }
//     }
// }

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    esp_println::println!("Init!");

    // let peripherals = esp_hal::init(esp_hal::Config::default());
    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    esp_alloc::heap_allocator!(72 * 1024); // TODO is this too big!

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let output_toggle_pin = Output::new(io.pins.gpio2, Level::High);
    let button_pin = Input::new(io.pins.gpio1, Pull::Up);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timg1 = TimerGroup::new(peripherals.TIMG1);

    // Initialize the timers used for Wifi
    // TODO: can the embassy timers be used?
    //let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let init = init(
        EspWifiInitFor::Wifi,
        timg1.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let (wifi_device, mut controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    // TODO get ip address
    // Init network stack

    let config = embassy_net::Config::dhcpv4(Default::default());
    let seed = 1234; // very random, very secure seed  TODO use  esp_hal::rng::Rng

    let stack = &*STACK.init(embassy_net::Stack::new(
        wifi_device,
        config,
        RESOURCES.init(embassy_net::StackResources::new()),
        seed,
    ));

    esp_hal_embassy::init(timg0.timer0);

    spawner.spawn(run()).ok();
    spawner.spawn(wifi_connect(controller)).ok();
    spawner.spawn(run_network_stack(stack)).ok();
    spawner.spawn(toggle_pin(output_toggle_pin)).ok();
    spawner.spawn(button_monitor(button_pin)).ok();
    spawner.spawn(bing()).ok();
    spawner.spawn(access_web(stack)).ok();
}
