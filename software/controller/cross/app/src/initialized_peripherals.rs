// Based on https://users.rust-lang.org/t/how-to-borrow-peripherals-struct/83565/3

use esp_hal::gpio::{Input, Level, Output, Pull};
use esp_hal::peripherals::{Peripherals, RADIO_CLK, SPI2, TIMG1, WIFI};
use esp_hal::rng::Rng;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;

use esp_wifi::wifi::{WifiController, WifiDevice, WifiStaDevice};
use esp_wifi::{init, EspWifiController};

use embassy_net::{Runner, Stack};

use static_cell::StaticCell;

use crate::constants::NUMBER_SOCKETS_STACK_RESOURCES;

static ESP_WIFI_CONTROLLER: StaticCell<EspWifiController<'static>> = StaticCell::new();

static RESOURCES: StaticCell<embassy_net::StackResources<NUMBER_SOCKETS_STACK_RESOURCES>> =
    StaticCell::new();

pub struct InitilizedPeripherals {
    pub button_pin: Input<'static>,
    pub sclk: Output<'static>,
    pub mosi: Output<'static>,
    pub miso: Output<'static>,
    pub xcs: Output<'static>,
    pub xdcs: Output<'static>,
    pub dreq: Input<'static>,
    pub reset: Output<'static>,
    // pub rng: Rng,
    pub system_timer: SystemTimer,

    pub spi2: SPI2,

    pub sta_stack: Stack<'static>,
    pub runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>,
    pub wifi_controller: WifiController<'static>,
}

impl InitilizedPeripherals {
    pub fn init<const NUMBER_SOCKETS_STACK_RESOURCES: usize>(
        peripherals: Peripherals,
    ) -> InitilizedPeripherals {
        let rng = Rng::new(peripherals.RNG);

        let wifi = peripherals.WIFI;
        let radio_clk = peripherals.RADIO_CLK;
        let systimer = peripherals.SYSTIMER;

        let timg1 = TimerGroup::new(peripherals.TIMG1);

        let spi2 = peripherals.SPI2;

        let wifi_peripherals = WifiInitializedPeripherals::init_wifi::<
            NUMBER_SOCKETS_STACK_RESOURCES,
        >(wifi, radio_clk, timg1, rng.clone());
        InitilizedPeripherals {
            button_pin: Input::new(peripherals.GPIO1, Pull::Up),
            sclk: Output::new(peripherals.GPIO5, Level::Low),
            mosi: Output::new(peripherals.GPIO6, Level::Low),
            miso: Output::new(peripherals.GPIO7, Level::Low),
            xcs: Output::new(peripherals.GPIO9, Level::Low),
            xdcs: Output::new(peripherals.GPIO10, Level::Low),
            dreq: Input::new(peripherals.GPIO8, Pull::None),
            reset: Output::new(peripherals.GPIO20, Level::High),

            system_timer: SystemTimer::new(systimer),
            // SPI
            spi2,

            // Peripherals required for wifi
            sta_stack: wifi_peripherals.sta_stack,
            runner: wifi_peripherals.runner,
            wifi_controller: wifi_peripherals.wifi_controller,
        }
    }
}

struct WifiInitializedPeripherals {
    pub sta_stack: Stack<'static>,
    pub runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>,
    pub wifi_controller: WifiController<'static>,
}

impl WifiInitializedPeripherals {
    pub fn init_wifi<const NUMBER_SOCKETS_STACK_RESOURCES: usize>(
        wifi: WIFI,
        radio_clk: RADIO_CLK,
        timg: TimerGroup<TIMG1>,
        rng: Rng,
    ) -> Self {
        let mut esp32_rng = rng;

        let init = ESP_WIFI_CONTROLLER.uninit().write(
            init(
                timg.timer0,
                //Rng::new(peripherals.RNG.clone()),
                esp32_rng.clone(),
                radio_clk,
            )
            .unwrap(),
        );

        let (wifi_device, controller) =
            esp_wifi::wifi::new_with_mode(init, wifi, WifiStaDevice).unwrap();

        let sta_config = embassy_net::Config::dhcpv4(Default::default());

        // Random seed.
        // Taken from example line 104 https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point_with_sta.rs
        let seed = (esp32_rng.random() as u64) << 32 | esp32_rng.random() as u64;

        // Init network stacks
        let (sta_stack, sta_runner) = embassy_net::new(
            wifi_device,
            sta_config,
            RESOURCES.init(embassy_net::StackResources::new()), // mk_static!(StackResources<3>, StackResources::<3>::new()),
            //&mut embassy_net::StackResources::<NUMBER_SOCKETS_STACK_RESOURCES>::new(), // mk_static!(StackResources<3>, StackResources::<3>::new()),
            seed,
        );
        Self {
            sta_stack,
            runner: sta_runner,
            wifi_controller: controller,
        }
    }
}
