//! embassy hello world
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

//% CHIPS: esp32 esp32c2 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3
//% FEATURES: embassy esp-hal-embassy/integrated-timers

#![no_std]
#![no_main]

use embassy_executor::Spawner;
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

use embedded_io::*;
use esp_wifi::wifi::WifiApDevice;
use esp_wifi::{
    init,
    wifi::{
        utils::create_network_interface, AccessPointInfo, AuthMethod, ClientConfiguration,
        Configuration, WifiError, WifiMode, WifiStaDevice,
    },
    wifi_interface::WifiStack,
    EspWifiInitFor,
};
use smoltcp::iface::SocketStorage;
use smoltcp::wire::IpAddress;
use smoltcp::wire::Ipv4Address;

const SSID: &str = env!("WLAN-SSID");
const PASSWORD: &str = env!("WLAN-PASSWORD");

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
        esp_println::println!("Button pressed!");
    }
}

#[embassy_executor::task]
async fn bing() {
    loop {
        esp_println::println!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}

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

    esp_alloc::heap_allocator!(72 * 1024);

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

    // Configure wifi
    let mut wifi = peripherals.WIFI;
    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let (wifi_interface, wifi_device, mut controller, sockets) =
        create_network_interface(&init, &mut wifi, WifiStaDevice, &mut socket_set_entries).unwrap();

    // Client config start
    let mut auth_method = AuthMethod::WPA2Personal;
    if PASSWORD.is_empty() {
        auth_method = AuthMethod::None;
    }

    let client_config = Configuration::Client(ClientConfiguration {
        // ANCHOR_END: client_config_start
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        auth_method,          // TODO: Is AuthMethod::WPA2Personal the default?
        ..Default::default()  // ANCHOR: client_config_end
    });
    esp_println::println!("SSID: {}", SSID);
    esp_println::println!("PASSWORD: {}", PASSWORD);

    // let client_config = Configuration::Client(.....);
    let res = controller.set_configuration(&client_config);
    esp_println::println!("Wi-Fi set_configuration returned {:?}", res);

    controller.start().unwrap();
    esp_println::println!("Is wifi started: {:?}", controller.is_started());

    esp_println::println!("Start WiFi Scan");
    let res: Result<(heapless::Vec<AccessPointInfo, 10>, usize), WifiError> = controller.scan_n();
    esp_println::println!("Scan result:{:?}", res); // <------ Err

    if let Ok((res, _count)) = res {
        for ap in res {
            //esp_println::println!("AP:{:?}", ap);
            esp_println::println!("AP SSID {}, CHANNEL {}", ap.ssid, ap.channel);
        }
    }

    esp_println::println!("{:?}", controller.get_capabilities());
    esp_println::println!("Wi-Fi connect: {:?}", controller.connect());

    // Wait to get connected
    esp_println::println!("Wait to get connected");
    loop {
        let res = controller.is_connected();
        match res {
            Ok(connected) => {
                if connected {
                    break;
                }
            }
            Err(err) => {
                esp_println::println!("{:?}", err);
                loop {}
            }
        }
    }
    esp_println::println!("{:?}", controller.is_connected());

    // TODO get ip address

    esp_hal_embassy::init(timg0.timer0);

    spawner.spawn(run()).ok();
    spawner.spawn(toggle_pin(output_toggle_pin)).ok();
    spawner.spawn(button_monitor(button_pin)).ok();
    spawner.spawn(bing()).ok();
}
