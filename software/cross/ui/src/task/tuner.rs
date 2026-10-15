use embassy_time::{Duration, Timer};

use crate::front_panel::{Buttons, FrontPanel};
use crate::task::sync::{TunerDirection, TunerEvent, TunerEventsChannelPublisher};
use radio_control_protocol::StationConfig;

#[allow(unused_imports)]
use log::{debug, error, info};

use periodic_map::PeriodicMap;

mod tuning_scale;

use tuning_scale::TuningScale;

const VALID_WINDOW: usize = 5;
const INVALID_WINDOW: usize = 10;

// DESIGN NOTE: This does not debounce the buttons in the traditional way,
// but this polling technique seems to work just fine.
#[embassy_executor::task]
pub async fn tuner(
    front_panel: &'static FrontPanel,
    stations_config: &'static StationConfig,
    //tuner_events_publisher: &'static mut TunerEventsChannelPublisher,
    tuner_events_publisher: TunerEventsChannelPublisher,
) {
    // Use as the initial station :
    // 1. The last set station - TODO
    // 2. The first preset stations if set
    // 3. The first station in the station list
    let initial_station = stations_config.map_preset(0).map_or(0, |o| o);

    let number_stations = stations_config.number_stations;

    // Send the inital station
    tuner_events_publisher
        .publish(TunerEvent::StationChanged(Some(initial_station)))
        .await;

    let mut last_button_pressed = Buttons::None;

    let mut last_station_id = None;

    // Intrepretating rotary encoder movement to as a tuning scale as used in an old analog radio
    let mut tuning_scale = TuningScale::new(number_stations * (VALID_WINDOW + INVALID_WINDOW));
    // Used for mapping the rotary encoder to stations. Is safe as long as VALID_WINDOW and
    // INVALID_WINDOW are more than 0.
    let periodic_map = PeriodicMap::new(VALID_WINDOW, INVALID_WINDOW).unwrap();

    loop {
        let button_pressed = match front_panel.button_pressed().await {
            Ok(button) => button,
            // Sometimes the multiplexer misfires. If so ignore this.
            Err(_) => continue,
        };

        if button_pressed != last_button_pressed {
            last_button_pressed = button_pressed.clone();

            //let stations = station_config.lock().await;

            let selection = match button_pressed {
                Buttons::RotaryEncoderSwitch => None,

                Buttons::Button1 => stations_config.map_preset(0),
                Buttons::Button3 => stations_config.map_preset(2),
                Buttons::Button4 => stations_config.map_preset(3),
                Buttons::Button2 => stations_config.map_preset(1),
                Buttons::None => None, // No button pressed so keep waiting
                Buttons::Unknown => {
                    error!("Unknown button pressed");
                    None
                }
            };

            // [ ] test what happens when no preset is set!

            if let Some(station_id) = selection {
                info!("Playing preset station: {}\n\n", station_id);

                // Adjust the tuner scale so that any later movement is from the selected preset station
                let scale_value = periodic_map.inverse_map(station_id);
                tuning_scale.set(scale_value);

                // Signal that the station has changed
                //station_change_sender.send(Some(station_id));
                tuner_events_publisher
                    .publish(TunerEvent::StationChanged(Some(station_id)))
                    .await;
            }
        }

        // Now read the rotary encoder.
        let mut rotary_encoder_movement = false;
        let direction = front_panel.decode_rotary_encoder().await.unwrap();
        match direction {
            crate::front_panel::Direction::Clockwise => {
                rotary_encoder_movement = true;
                tuning_scale.increment();
                tuner_events_publisher
                    .publish(TunerEvent::TunerMoved(TunerDirection::Clockwise))
                    .await;
            }
            crate::front_panel::Direction::CounterClockwise => {
                rotary_encoder_movement = true;
                tuning_scale.decrement();
                tuner_events_publisher
                    .publish(TunerEvent::TunerMoved(TunerDirection::CounterClockwise))
                    .await;
            }
            crate::front_panel::Direction::None => (),
        }

        if rotary_encoder_movement {
            let station_id = periodic_map.map(tuning_scale.get());

            if station_id != last_station_id {
                match station_id {
                    Some(id) => {
                        info!("Playing station: {:?}\n\n", id);
                        last_station_id = station_id;
                        //station_change_sender.send(Some(id));
                        tuner_events_publisher
                            .publish(TunerEvent::StationChanged(Some(id)))
                            .await;

                        // TODO assuming that the following will work.
                        //stations.set_current_station(id).unwrap();
                    }
                    None => {
                        //stations.set_current_station(None);
                        last_station_id = station_id;
                        //station_change_sender.send(None);
                        tuner_events_publisher
                            .publish(TunerEvent::StationChanged(None))
                            .await;
                    }
                }
            }
        }

        Timer::after(Duration::from_millis(5)).await;
    }
}
