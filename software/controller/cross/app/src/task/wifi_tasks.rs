// Wifi secrets stored as environment varaibles
const SSID: &str = env!("WLAN-SSID");
const PASSWORD: &str = env!("WLAN-PASSWORD");

use esp_wifi::wifi::{
    AuthMethod, ClientConfiguration, Configuration, WifiController, WifiDevice, WifiStaDevice,
};

use embassy_net::Runner;

use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn wifi_connect(mut controller: WifiController<'static>) {
    esp_println::println!("Wait to get wifi connected");

    loop {
        if !matches!(controller.is_started(), Ok(true)) {
            let mut auth_method = AuthMethod::WPA2Personal;
            #[allow(clippy::const_is_empty)]
            if PASSWORD.is_empty() {
                auth_method = AuthMethod::None;
            }

            let wifi_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                auth_method, // TODO: Is AuthMethod::WPA2Personal the default?
                ..Default::default()
            });
            let res = controller.set_configuration(&wifi_config);
            esp_println::println!("Wi-Fi set_configuration returned {:?}", res);

            esp_println::println!("Starting wifi");
            controller.start_async().await.unwrap();
            esp_println::println!("Wifi started!");
        }

        match controller.connect_async().await {
            Ok(_) => esp_println::println!("Wifi connected!"),
            Err(e) => {
                esp_println::println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

// Run the network stack.
// This must be called in a background task, to process network events.
#[embassy_executor::task]
pub async fn run_network_stack(mut runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) {
    runner.run().await
}
