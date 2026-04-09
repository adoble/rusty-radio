/// The contains the initialised peripherals used by the system.
// Based on https://users.rust-lang.org/t/how-to-borrow-peripherals-struct/83565/3
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    interrupt::software::{SoftwareInterrupt, SoftwareInterruptControl},
    peripherals::Peripherals,
    spi::master::{Config as SpiConfig, Spi},
    timer::systimer::SystemTimer,
};

pub struct Hardware {
    pub mux_cs: Output<'static>,

    #[allow(dead_code)]
    pub disp_cs: Output<'static>,

    pub system_timer: SystemTimer<'static>,
    //pub rng: Rng,
    pub spi_bus_ui: Spi<'static, esp_hal::Async>,

    // Required to setup embassy/esp-rtos
    pub software_interrupt0: SoftwareInterrupt<'static, 0>,
    //pub timer_group: TimerGroup<'static, TIMG1<'static>>,
}

impl Hardware {
    pub fn init(peripherals: Peripherals) -> Hardware {
        let systimer = peripherals.SYSTIMER;

        // Create the SPI from the HAL. This implements SpiBus, not SpiDevice!
        // Only SPI2 is available for the ESP32-C3 - TODO is this true?
        let spi_bus_ui: Spi<'_, esp_hal::Async> = Spi::new(peripherals.SPI2, SpiConfig::default())
            .expect("PANIC: Could not initialize UI SPI")
            .with_sck(peripherals.GPIO8)
            .with_mosi(peripherals.GPIO10)
            .with_miso(peripherals.GPIO9)
            .into_async();

        let output_config = OutputConfig::default();

        Hardware {
            // Multiplexoer CS for the button board
            mux_cs: Output::new(peripherals.GPIO3, Level::High, output_config),
            // Intially set high to display the diplay.
            disp_cs: Output::new(peripherals.GPIO2, Level::High, output_config),

            //disp_cs: Output::new(peripherals.GPIO8, Level::High, output_config),

            // Assuming that the interrupt signal is actively driven and not open drain.
            // intr: Input::new(
            //     peripherals.GPIO3,
            //     InputConfig::default().with_pull(Pull::None),
            // ),
            system_timer: SystemTimer::new(systimer),

            // rng,

            // SPI
            spi_bus_ui,

            // Required to initialise embassy over esp-rtos
            // timer_group,
            software_interrupt0: SoftwareInterruptControl::new(peripherals.SW_INTERRUPT)
                .software_interrupt0,
        }
    }
}
