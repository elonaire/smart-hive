use core::time::Duration;

use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::sys::EspError;

use log::*;

pub fn create_event_loop<F>(
    client: &mut EspMqttClient<'_>,
    connection: &mut EspMqttConnection,
    topic: &str,
    mut on_message: F,
) -> Result<(), EspError>
where
    F: FnMut(&str) + Send + 'static,
{
    std::thread::scope(|s| {
        info!("About to start the MQTT client");

        std::thread::Builder::new()
            .stack_size(6000)
            .spawn_scoped(s, move || {
                info!("MQTT Listening for messages");

                while let Ok(event) = connection.next() {
                    match event.payload() {
                        EventPayload::Received { data, .. } => {
                            if let Ok(payload) = std::str::from_utf8(data) {
                                info!("[Queue] Received: {}", payload);
                                on_message(payload);
                            } else {
                                warn!("Received non-UTF8 payload");
                            }
                        }
                        EventPayload::Connected(_) => {
                            info!("MQTT Connected");
                        }
                        EventPayload::Disconnected => {
                            warn!("MQTT Disconnected");
                        }
                        _ => {}
                    }
                }

                info!("Connection closed");
            })
            .unwrap();

        loop {
            if let Err(e) = client.subscribe(topic, QoS::AtMostOnce) {
                error!("Failed to subscribe to topic \"{topic}\": {e}, retrying...");
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }

            info!("Subscribed to topic \"{topic}\"");
            std::thread::sleep(Duration::from_millis(500));

            // Keep the main thread alive
            loop {
                std::thread::sleep(Duration::from_secs(60));
            }
        }
    })
}