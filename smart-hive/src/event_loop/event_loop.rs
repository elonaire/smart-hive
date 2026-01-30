use core::time::Duration;

use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::sys::EspError;

use log::*;

pub fn create_event_loop<F>(
    client: &mut EspMqttClient<'_>,
    connection: &mut EspMqttConnection,
    mut on_message: F,
) -> Result<(), EspError>
where
    F: FnMut(Option<&str>, &str) + Send + 'static,  // (topic, payload)
{
    std::thread::scope(|s| {
        info!("Starting the MQTT client!");

        std::thread::Builder::new()
            .stack_size(6000)
            .spawn_scoped(s, move || {
                info!("MQTT Listening for messages");

                while let Ok(event) = connection.next() {
                    match event.payload() {
                        EventPayload::Received { topic, data, .. } => {
                            if let Ok(payload) = std::str::from_utf8(data) {
                                info!("[{}] Received: {}", topic, payload);
                                on_message(topic, payload);
                            } else {
                                warn!("Received non-UTF8 payload on topic: {}", topic);
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

        // Keep main thread alive
        loop {
            std::thread::sleep(Duration::from_secs(60));
        }
    })
}