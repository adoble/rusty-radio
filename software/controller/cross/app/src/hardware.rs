use embassy_time::Timer;
/// The contains the initialised peripherals used by the system.
// Based on https://users.rust-lang.org/t/how-to-borrow-peripherals-struct/83565/3
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    // peripherals::{Peripherals, RADIO_CLK, TIMG1, WIFI},
    peripherals::{Peripherals, TIMG1, WIFI},
    rng::Rng,
    spi::master::{Config as SpiConfig, Spi},
    timer::{systimer::SystemTimer, timg::TimerGroup},
};

use esp_wifi::wifi::{WifiController, WifiDevice};
use esp_wifi::{init, EspWifiController};

use embassy_net::{Runner, Stack};

use static_cell::StaticCell;

use crate::constants::NUMBER_SOCKETS_STACK_RESOURCES;

static ESP_WIFI_CONTROLLER: StaticCell<EspWifiController<'static>> = StaticCell::new();

static RESOURCES: StaticCell<embassy_net::StackResources<NUMBER_SOCKETS_STACK_RESOURCES>> =
    StaticCell::new();

pub struct Hardware {
    //pub button_pin: Input<'static>,
    pub xcs: Output<'static>,
    pub xdcs: Output<'static>,
    pub dreq: Input<'static>,
    pub reset: Output<'static>,
    pub mux_cs: Output<'static>,
    //pub led: Output<'static>,
    pub intr: Input<'static>,
    // pub rng: Rng,
    pub system_timer: SystemTimer<'static>,

    //pub spi2: SPI2,
    pub spi_bus: Spi<'static, esp_hal::Async>,

    pub sta_stack: Stack<'static>,
    pub runner: Runner<'static, WifiDevice<'static>>,
    // pub wifi_controller: WifiController<'static>,
    pub wifi_controller: &'static mut WifiController<'static>,
}

impl Hardware {
    pub fn init<const NUMBER_SOCKETS_STACK_RESOURCES: usize>(peripherals: Peripherals) -> Hardware {
        let rng = Rng::new(peripherals.RNG);

        let wifi = peripherals.WIFI;
        //let radio_clk = peripherals.RADIO_CLK;
        let systimer = peripherals.SYSTIMER;

        let timg1: TimerGroup<'_, _> = TimerGroup::new(peripherals.TIMG1);

        // Create the SPI from the HAL. This implements SpiBus, not SpiDevice!
        // Only SPI2 is available for the ESP32-C3 - TODO is this true?
        let spi_bus: Spi<'_, esp_hal::Async> = Spi::new(peripherals.SPI2, SpiConfig::default())
            .expect("PANIC: Could not initialize SPI")
            .with_sck(peripherals.GPIO5)
            .with_mosi(peripherals.GPIO6)
            .with_miso(peripherals.GPIO7)
            .into_async();

        let output_config = OutputConfig::default();
        // let input_config = InputConfig::default();

        let wifi_peripherals =
            WifiHardware::init_wifi::<NUMBER_SOCKETS_STACK_RESOURCES>(wifi, timg1, rng);
        Hardware {
            //button_pin: Input::new(peripherals.GPIO9, Pull::Up),
            mux_cs: Output::new(peripherals.GPIO2, Level::High, output_config),

            xcs: Output::new(peripherals.GPIO4, Level::High, output_config),
            xdcs: Output::new(peripherals.GPIO10, Level::High, output_config),
            dreq: Input::new(
                peripherals.GPIO8,
                InputConfig::default().with_pull(Pull::None),
            ),
            reset: Output::new(peripherals.GPIO20, Level::High, output_config),
            //led: Output::new(peripherals.GPIO3, Level::Low),

            // Assuming that the interrupt signal is actively driven and not open drain.
            intr: Input::new(
                peripherals.GPIO3,
                InputConfig::default().with_pull(Pull::None),
            ),

            system_timer: SystemTimer::new(systimer),
            // SPI
            spi_bus,

            // Peripherals required for wifi
            sta_stack: wifi_peripherals.sta_stack,
            runner: wifi_peripherals.runner,
            wifi_controller: wifi_peripherals.wifi_controller,
        }
    }
}

struct WifiHardware {
    pub sta_stack: Stack<'static>,
    pub runner: Runner<'static, WifiDevice<'static>>,
    pub wifi_controller: &'static mut WifiController<'static>,
}

impl WifiHardware {
    // Based on the example here: https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point_with_sta.rs#L301
    pub fn init_wifi<const NUMBER_SOCKETS_STACK_RESOURCES: usize>(
        wifi: WIFI<'static>,
        //radio_clk: RADIO_CLK,
        timg: TimerGroup<'static, TIMG1>,
        mut rng: Rng,
    ) -> Self {
        // let esp_wifi_ctrl = ESP_WIFI_CONTROLLER.uninit().write(
        //     init(
        //         timg.timer0,
        //         //Rng::new(peripherals.RNG.clone()),
        //         rng.clone(),
        //         radio_clk,
        //     )
        //     .unwrap(),
        // );

        let esp_wifi_ctrl =
            ESP_WIFI_CONTROLLER.init(esp_wifi::init(timg.timer0, rng.clone()).unwrap());

        let (controller, interfaces) = esp_wifi::wifi::new(esp_wifi_ctrl, wifi).unwrap();
        let wifi_device = interfaces.sta;

        // let (wifi_device, controller) =
        //     esp_wifi::wifi::new_with_mode(esp_wifi_ctrl, wifi, WifiStaDevice).unwrap();

        let sta_config = embassy_net::Config::dhcpv4(Default::default());

        // Random seed.
        // Taken from example line 104 https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point_with_sta.rs
        let seed = ((rng.random() as u64) << 32) | rng.random() as u64;

        // Init network stacks
        let (sta_stack, sta_runner) = embassy_net::new(
            wifi_device,
            sta_config,
            RESOURCES.init(embassy_net::StackResources::new()),
            seed,
        );

        // Make the controller static
        static CONTROLLER: StaticCell<WifiController> = StaticCell::new();
        let controller = CONTROLLER.init(controller);

        Self {
            sta_stack,
            runner: sta_runner,
            wifi_controller: controller,
        }
    }
}
