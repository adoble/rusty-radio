// Wifi secrets stored as environment varaibles
const SSID: &str = env!("WLAN_SSID");
const PASSWORD: &str = env!("WLAN_PASSWORD");

use esp_radio::wifi::{AuthMethod, ClientConfig, ModeConfig, WifiController, WifiDevice};

use embassy_net::Runner;

use embassy_time::{Duration, Timer};

use crate::task::sync::WIFI_CONNECTED_SIGNAL;

use log::{error, info};

#[embassy_executor::task]
pub async fn wifi_connect(controller: &'static mut WifiController<'static>) {
    loop {
        if !matches!(controller.is_started(), Ok(true)) {
            let mut auth_method = AuthMethod::Wpa2Personal;
            #[allow(clippy::const_is_empty)]
            if PASSWORD.is_empty() {
                auth_method = AuthMethod::None;
            }

            // For esp_radio
            let wifi_config = ModeConfig::Client(
                ClientConfig::default()
                    .with_ssid(SSID.into())
                    .with_password(PASSWORD.into())
                    .with_auth_method(auth_method),
            );

            // let res = controller.set_configuration(&wifi_config);
            let _ = controller.set_config(&wifi_config);

            controller.start_async().await.unwrap();
            info!("Wifi started!");
        }

        match controller.connect_async().await {
            Ok(_) => {
                info!("Wifi connected!");
                WIFI_CONNECTED_SIGNAL.signal(true)
            }
            Err(e) => {
                error!("Failed to connect to wifi: {e:?}");
                WIFI_CONNECTED_SIGNAL.signal(false);
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

// Run the network stack.
// This must be called in a background task, to process network events.
#[embassy_executor::task]
pub async fn run_network_stack(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
