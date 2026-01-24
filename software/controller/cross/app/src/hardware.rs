//use embassy_time::Timer;
/// The contains the initialised peripherals used by the system.
// Based on https://users.rust-lang.org/t/how-to-borrow-peripherals-struct/83565/3
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    interrupt::software::{SoftwareInterrupt, SoftwareInterruptControl},
    peripherals::{Peripherals, TIMG1, WIFI},
    rng::Rng,
    spi::master::{Config as SpiConfig, Spi},
    timer::{systimer::SystemTimer, timg::TimerGroup},
};

//use esp_wifi::wifi::{WifiController, WifiDevice};
use esp_radio::{
    wifi::{WifiController, WifiDevice},
    Controller,
};
// use esp_wifi::EspWifiController;

use esp_println::println;

use embassy_net::{Runner, Stack};

use static_cell::StaticCell;

use crate::constants::NUMBER_SOCKETS_STACK_RESOURCES;

//static ESP_WIFI_CONTROLLER: StaticCell<EspWifiController<'static>> = StaticCell::new();
static ESP_WIFI_CONTROLLER: StaticCell<Controller<'static>> = StaticCell::new();

static RESOURCES: StaticCell<embassy_net::StackResources<NUMBER_SOCKETS_STACK_RESOURCES>> =
    StaticCell::new();

pub struct Hardware {
    //pub button_pin: Input<'static>,
    pub xcs: Output<'static>,
    pub xdcs: Output<'static>,
    pub dreq: Input<'static>,
    pub reset_codec: Output<'static>,
    pub mux_cs: Output<'static>,
    pub disp_cs: Output<'static>,

    pub system_timer: SystemTimer<'static>,
    pub timer_group: TimerGroup<'static, TIMG1<'static>>,
    pub rng: Rng,

    pub spi_bus: Spi<'static, esp_hal::Async>,

    pub software_interrupt0: SoftwareInterrupt<'static, 0>,
    // pub sta_stack: Stack<'static>,
    // pub runner: Runner<'static, WifiDevice<'static>>,
    // pub wifi_controller: &'static mut WifiController<'static>,
    pub wifi: WIFI<'static>,
}

impl Hardware {
    pub fn init<const NUMBER_SOCKETS_STACK_RESOURCES: usize>(peripherals: Peripherals) -> Hardware {
        // let rng = Rng::new(peripherals.RNG);
        let rng = Rng::new();

        //let wifi = peripherals.WIFI;
        //let radio_clk = peripherals.RADIO_CLK;
        let systimer = peripherals.SYSTIMER;

        let timer_group = TimerGroup::new(peripherals.TIMG1);

        // Create the SPI from the HAL. This implements SpiBus, not SpiDevice!
        // Only SPI2 is available for the ESP32-C3 - TODO is this true?
        let spi_bus: Spi<'_, esp_hal::Async> = Spi::new(peripherals.SPI2, SpiConfig::default())
            .expect("PANIC: Could not initialize SPI")
            .with_sck(peripherals.GPIO4)
            .with_mosi(peripherals.GPIO5)
            .with_miso(peripherals.GPIO6)
            .into_async();

        let output_config = OutputConfig::default();

        Hardware {
            // Pins for ESP32-S6
            mux_cs: Output::new(peripherals.GPIO1, Level::High, output_config),

            xcs: Output::new(peripherals.GPIO3, Level::High, output_config),
            xdcs: Output::new(peripherals.GPIO9, Level::High, output_config),
            dreq: Input::new(
                peripherals.GPIO7,
                InputConfig::default().with_pull(Pull::None),
            ),
            //reset: Output::new(peripherals.GPIO20, Level::High, output_config),
            reset_codec: Output::new(peripherals.GPIO2, Level::High, output_config),

            // Intially set high to display the diplay.
            disp_cs: Output::new(peripherals.GPIO8, Level::High, output_config),

            // Assuming that the interrupt signal is actively driven and not open drain.
            // intr: Input::new(
            //     peripherals.GPIO3,
            //     InputConfig::default().with_pull(Pull::None),
            // ),
            system_timer: SystemTimer::new(systimer),

            timer_group,

            rng,
            // SPI
            spi_bus,

            // Required to initialise embassy over esp-rtos
            software_interrupt0: SoftwareInterruptControl::new(peripherals.SW_INTERRUPT)
                .software_interrupt0,
            // // Peripherals required for wifi
            // sta_stack: wifi_peripherals.sta_stack,
            // runner: wifi_peripherals.runner,
            // wifi_controller: wifi_peripherals.wifi_controller,
            wifi: peripherals.WIFI,
        }
    }
}

pub struct WifiHardware {
    pub sta_stack: Stack<'static>,
    pub runner: Runner<'static, WifiDevice<'static>>,
    pub wifi_controller: &'static mut WifiController<'static>,
}

impl WifiHardware {
    // Based on the example here: https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point_with_sta.rs#L301
    pub fn init_wifi<const NUMBER_SOCKETS_STACK_RESOURCES: usize>(
        wifi: WIFI<'static>,
        //radio_clk: RADIO_CLK,
        //timg: TimerGroup<'static, TIMG1>,
        rng: Rng,
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

        // let esp_wifi_ctrl =
        //     ESP_WIFI_CONTROLLER.init(esp_radio::init(timg.timer0, rng.clone()).unwrap());

        let res = esp_radio::init();

        let esp_wifi_ctrl = match res {
            Ok(controller) => ESP_WIFI_CONTROLLER.init(controller),
            Err(e) => panic!("ERROR: wifi controller not initialised: {:?}", e),
        };

        // TODO reinstate
        //let esp_wifi_ctrl = ESP_WIFI_CONTROLLER.init(esp_radio::init().unwrap());

        let (controller, interfaces) =
            esp_radio::wifi::new(esp_wifi_ctrl, wifi, Default::default()).unwrap();

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
