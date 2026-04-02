// TODO remove
#![allow(deprecated)]
use crate::{front_panel::Buttons, FrontPanel};

use esp_hal::gpio::Input;

// TODO test out using fugit or something similar to set a time duration

// CHECKLIST
// [X]  test out a single button press. This works!
// [ ]  test the interrupt
// [ ]  test descriminating the button

#[embassy_executor::task]
#[deprecated]
pub async fn test_button_board(
    front_panel: &'static FrontPanel,
    mut _interrupt_pin: Input<'static>,
) {
    // TODO should the multiplexer be set up here?

    // TEST code
    let mut last_button_pressed = Buttons::None;

    loop {
        let button_pressed = front_panel.button_pressed().await.unwrap();

        if button_pressed != last_button_pressed {
            last_button_pressed = button_pressed.clone();
        }
    }
}
