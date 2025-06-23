use embassy_time::{Duration, Timer};

use crate::{front_panel::Buttons, FrontPanel};

use esp_hal::gpio::Input;

const DEBOUNCE_DURATION: u64 = 50;

// TODO test out using fugit or something similar to set a time duration

// CHECKLIST
// [X]  test out a single button press. This works!
// [ ]  test the interrupt
// [ ]  test descriminating the button

#[embassy_executor::task]
pub async fn test_button_board(
    front_panel: &'static FrontPanel,
    mut _interrupt_pin: Input<'static>,
) {
    // TODO should the multiplexer be set up here?

    // TEST code
    let mut last_button_pressed = Buttons::None;

    loop {
        //interrupt_pin.wait_for_rising_edge().await;
        //esp_println::println!("DEBUG: Interrupt pin low");

        let button_pressed = front_panel.button_pressed().await.unwrap();

        if button_pressed != last_button_pressed {
            esp_println::println!("DEBUG: Button pressed = {:?}", button_pressed);
            last_button_pressed = button_pressed.clone();
        }

        // Timer::after(Duration::from_millis(DEBOUNCE_DURATION)).await;

        // let button_still_pressed: bool = match button_pressed {
        //     Buttons::Button1 => front_panel.read_button(&button_pressed).await.unwrap(),
        //     _ => false, //Ignore
        // };

        // if button_still_pressed {
        //     esp_println::println!("DEBUG: Button pressed:  {:?}", button_pressed);
        // }

        // // Read the button to clear the interrupt
        // let button_1_pressed = front_panel.read_button(Buttons::Button1).await.unwrap();

        // // Debounce
        // Timer::after(Duration::from_millis(DEBOUNCE_DURATION)).await;
        // if button_1_pressed && front_panel.read_button(Buttons::Button1).await.unwrap() {
        //     // Button is still pressed so acknowledge
        //     esp_println::println!("DEBUG: Button 1 pressed");
        // }
    }
}
