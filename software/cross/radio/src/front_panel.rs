use mcp23s17_async::{InterruptMode, Mcp23s17SpiError, PinMode};

use crate::task::sync::MULTIPLEXER_DRIVER;

use core::sync::atomic::{AtomicU8, Ordering};

// Map of the pins used in by the MCP23S17 on the button board
const ROT_A: u8 = 0;
const ROT_B: u8 = 1;
const ROT_SW: u8 = 2;
const BTN_1: u8 = 3;
const BTN_2: u8 = 4;
const BTN_3: u8 = 5;
const BTN_4: u8 = 6;

const LED: u8 = 8;

// The state of the rotary encoder: Needs to be maintained between task switching.
static ROTARY_ENCODER_STATE: AtomicU8 = AtomicU8::new(0);

/// The buttons/switches on the front panel
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Buttons {
    RotaryEncoderSwitch,
    Button1,
    Button2,
    Button3,
    Button4,
    None,
    Unknown,
}

/// The rotary encoder direction is either `Clockwise`, `CounterClockwise`, or `None`
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
    None,
}

// pub struct FrontPanel<SPI>
// where
//     SPI: SpiDevice,
// {
//     multiplexer_driver: Mcp23s17<SPI>,
// }

pub struct FrontPanel {
    //multiplexer_driver_mutex: &'static MultiplexerDriverMutex<'static>,
    // MultiplexerDriverType<'a>,
}

// We need to share the front panel driver between tasks so put it in a static mutex
// pub static MULTIPLEXER_DRIVER: Mutex<
//     CriticalSectionRawMutex,
//     Option<MultiplexerDriverType<'static>>,
// > = Mutex::new(None);

//TODO uses the global static MULTIPLEXER_DRIVER. Change to use a local variable (as parameter?).

impl FrontPanel {
    pub async fn new(//multiplexer_driver_mutex: MultiplexerDriverMutex<'static>,
    ) -> Result<Self, FrontPanelError> {
        //let multiplexer_driver_unlocked = MULTIPLEXER_DRIVER.lock().await;
        let mut multiplexer_driver_unlocked = MULTIPLEXER_DRIVER.lock().await;
        //let mut multiplexer_driver = Mcp23s17::new(spi, DEVICE_ADDR).await?;

        if let Some(multiplexer_driver) = multiplexer_driver_unlocked.as_mut() {
            // Set up the pin modes as required by the button_panel

            multiplexer_driver
                .pin_mode(ROT_A, PinMode::InputPullup)
                .await?;
            multiplexer_driver
                .pin_mode(ROT_B, PinMode::InputPullup)
                .await?;

            // Switch on button board is pulled down to GND.
            multiplexer_driver
                .pin_mode(ROT_SW, PinMode::InputFloating)
                .await?;

            multiplexer_driver
                .pin_mode(BTN_1, PinMode::InputPullup)
                .await?;
            multiplexer_driver
                .pin_mode(BTN_2, PinMode::InputPullup)
                .await?;
            multiplexer_driver
                .pin_mode(BTN_3, PinMode::InputPullup)
                .await?;
            multiplexer_driver
                .pin_mode(BTN_4, PinMode::InputPullup)
                .await?;

            multiplexer_driver.pin_mode(LED, PinMode::Output).await?;

            // Setup the interrupt modes
            multiplexer_driver
                .set_interrupt_mode(ROT_A, InterruptMode::ActiveLow)
                .await?;
            multiplexer_driver
                .set_interrupt_mode(ROT_B, InterruptMode::ActiveLow)
                .await?;

            multiplexer_driver
                .set_interrupt_mode(ROT_SW, InterruptMode::ActiveHigh)
                .await?;

            multiplexer_driver
                .set_interrupt_mode(BTN_1, InterruptMode::ActiveLow)
                .await?;
            multiplexer_driver
                .set_interrupt_mode(BTN_2, InterruptMode::ActiveLow)
                .await?;
            multiplexer_driver
                .set_interrupt_mode(BTN_3, InterruptMode::ActiveLow)
                .await?;
            multiplexer_driver
                .set_interrupt_mode(BTN_4, InterruptMode::ActiveLow)
                .await?;

            Ok(FrontPanel {
                //multiplexer_driver_mutex,
            })
        } else {
            Err(FrontPanelError::CannotGetMutex)
        }
    }

    #[allow(dead_code)]
    pub async fn set_led_high(&self) -> Result<(), FrontPanelError> {
        if let Some(multiplexer_driver) = MULTIPLEXER_DRIVER.lock().await.as_mut() {
            multiplexer_driver.set_high(LED).await?;
            Ok(())
        } else {
            Err(FrontPanelError::CannotGetMutex)
        }
    }

    #[allow(dead_code)]
    pub async fn set_led_low(&self) -> Result<(), FrontPanelError> {
        if let Some(multiplexer_driver) = MULTIPLEXER_DRIVER.lock().await.as_mut() {
            multiplexer_driver.set_low(LED).await?;
            Ok(())
        } else {
            Err(FrontPanelError::CannotGetMutex)
        }
    }

    /// Reads the button and returns `true` if the button is pressed
    #[allow(dead_code)]
    pub async fn read_button(&self, button: &Buttons) -> Result<bool, FrontPanelError> {
        let pin: u8 = match button {
            Buttons::RotaryEncoderSwitch => ROT_SW,
            Buttons::Button1 => BTN_1,
            Buttons::Button2 => BTN_2,
            Buttons::Button3 => BTN_3,
            Buttons::Button4 => BTN_4,
            Buttons::None => return Ok(false),
            Buttons::Unknown => return Ok(false),
        };

        if let Some(multiplexer_driver) = MULTIPLEXER_DRIVER.lock().await.as_mut() {
            // Button is active low
            let button_value = !multiplexer_driver.read(pin).await?;

            Ok(button_value)
        } else {
            Err(FrontPanelError::CannotGetMutex)
        }
    }

    /// Read the rotary encoder to determine which direction if was moved (it at all).
    ///
    /// This uses a [table based noise reducing digital filter algorithm](https://www.best-microcontroller-projects.com/rotary-encoder.html)
    ///
    /// **Note**
    ///
    /// This code is taken from the crate [rotary_encoder_hal](https://crates.io/crates/rotary-encoder-hal) `update` function
    /// (using a table) and modified to work with multiple threads.
    ///
    /// The crate could not directly be used as:
    ///  - it expects the rotary encoder to be connected directly to pins on the MCU and accessed
    ///    over `embedded_hal::digital::InputPin`. These are not available here as the rotary encoder is
    ///    connected to the multiplexer (MCP23S17) over SPI to the MCU.
    ///  - How it can handle multiple threads is not clear
    #[allow(dead_code)]
    pub async fn decode_rotary_encoder(&self) -> Result<Direction, FrontPanelError> {
        let (a, b) = self.read_rotary_encoder().await?;

        let mut state = ROTARY_ENCODER_STATE.load(Ordering::Relaxed);

        let mut prev_next = (state << 2) & 0xF;

        if a {
            prev_next |= 0x01;
        }

        if b {
            prev_next |= 0x02;
        }

        match prev_next {
            /*valid cases*/
            1 | 2 | 4 | 7 | 8 | 11 | 13 | 14 => {
                let result = (state & 0xF0) | prev_next;

                state = prev_next << 4 | prev_next;
                ROTARY_ENCODER_STATE.store(state, Ordering::Relaxed);

                Ok(Self::phase(result))
            }

            /*Invalid cases */
            0 | 3 | 5 | 6 | 9 | 10 | 12 | 15 => {
                state = state & 0xF0 | prev_next;
                ROTARY_ENCODER_STATE.store(state, Ordering::Relaxed);

                Ok(Direction::None)
            }

            /* let the compiler help us ensure we've covered them all */
            0x10..=0xFF => Ok(Direction::None),
        }
    }

    /// The useful values of `s` are:
    /// - 0x17 | 0x7E | 0xE8 | 0x81
    /// - 0x2B | 0xBD | 0xD4 | 0x42
    // fn phase(s: u8) -> Direction {
    //     //TODO why so few arms (see table above)?
    //     match s {
    //         0x17 => Direction::CounterClockwise,
    //         0x2b => Direction::Clockwise,
    //         _ => Direction::None,
    //     }
    // }
    fn phase(s: u8) -> Direction {
        //TODO why so few arms (see table above)?
        match s {
            0x17 => Direction::CounterClockwise,
            0x2B => Direction::Clockwise,
            _ => Direction::None,
        }
    }

    #[allow(dead_code)]
    pub async fn read_rotary_encoder(&self) -> Result<(bool, bool), FrontPanelError> {
        if let Some(multiplexer_driver) = MULTIPLEXER_DRIVER.lock().await.as_mut() {
            let a = multiplexer_driver.read(ROT_A).await?;
            let b = multiplexer_driver.read(ROT_B).await?;

            Ok((a, b))
        } else {
            Err(FrontPanelError::CannotGetMutex)
        }
    }

    #[allow(dead_code)]
    pub async fn read_rotary_controller_switch(&self) -> Result<bool, FrontPanelError> {
        if let Some(multiplexer_driver) = MULTIPLEXER_DRIVER.lock().await.as_mut() {
            let switch = multiplexer_driver.read(ROT_SW).await?;

            Ok(switch)
        } else {
            Err(FrontPanelError::CannotGetMutex)
        }
    }

    /// Which button was pressed. If more then one way pressed then lowest (leftmost) one
    /// is taken. Note that the rotary controller A dn B signals are not included
    pub async fn button_pressed(&self) -> Result<Buttons, FrontPanelError> {
        let (mut port_a, mut _port_b) =
            if let Some(multiplexer_driver) = MULTIPLEXER_DRIVER.lock().await.as_mut() {
                multiplexer_driver.read_all().await?
            } else {
                Err(FrontPanelError::CannotGetMutex)?
            };

        // pins 3-6 are active low so invert them (using xor)
        port_a ^= 0b0111_1000;

        // Pins 0 and 1 are the rotatary controller A and B signals and are ignored
        port_a &= 0b1111_1100;

        // esp_println::println!("port_a after partial inversion: {:08b}", port_a);

        let mut button_index = None;
        let mut mask: u8 = 0b00000001;
        for i in 0..8 {
            let pressed = (port_a & mask) >= 1;
            if pressed {
                button_index = Some(i);
                break;
            } else {
                mask <<= 1;
            }
        }

        // esp_println::println!("button index: {:?}", button_index);

        let button = match button_index {
            Some(2) => Buttons::RotaryEncoderSwitch,
            Some(3) => Buttons::Button1,
            Some(4) => Buttons::Button2,
            Some(5) => Buttons::Button3,
            Some(6) => Buttons::Button4,
            Some(7) => Buttons::Unknown,
            Some(_) => Buttons::Unknown,
            None => Buttons::None,
        };

        if button != Buttons::Unknown {
            Ok(button)
        } else {
            Err(FrontPanelError::UnknownInputControl)
        }
    }
}

// The driver may need to be shared (as a mutex) between different async tasks.
// Therefore the Send trait needs to be implemented (as a marker).
// See https://stackoverflow.com/questions/60292897/why-cant-i-send-mutexmut-c-void-between-threads
//unsafe impl<SPI> Send for FrontPanel<SPI> where SPI: SpiDevice {}

#[derive(Debug, Clone)]
pub enum FrontPanelError {
    Spi,
    UnknownInputControl,
    CannotGetMutex,
}

impl From<Mcp23s17SpiError> for FrontPanelError {
    fn from(_err: Mcp23s17SpiError) -> Self {
        Self::Spi
    }
}
