use core::time::Duration;

use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::sys::EspError;

use log::*;
use crate::MqttTopic;

pub fn create_event_loop<F>(
    client: &mut EspMqttClient<'_>,
    connection: &mut EspMqttConnection,
    mqtt_topics: &Vec<MqttTopic>,
    mut on_message: F,
) -> Result<(), EspError>
where
    F: FnMut(&str, &str) + Send + 'static,  // (topic, payload)
{
    std::thread::scope(|s| {
        info!("About to start the MQTT client");

        std::thread::Builder::new()
            .stack_size(8192)
            .spawn_scoped(s, move || {
                info!("MQTT Listening for messages");

                while let Ok(event) = connection.next() {
                    match event.payload() {
                        EventPayload::Received { topic, data, .. } => {
                            if let Ok(payload) = std::str::from_utf8(data) {
                                if let Some(topic_str) = topic {
                                    info!("[{}] Received: {}", topic_str, payload);
                                    on_message(topic_str, payload);
                                } else {
                                    warn!("Received message with no topic");
                                }
                            } else {
                                warn!("Received non-UTF8 payload on topic: {:?}", topic);
                            }
                        }
                        EventPayload::Connected(_) => {
                            info!("MQTT Connected");
                        }
                        EventPayload::Disconnected => {
                            warn!("MQTT Disconnected");
                        }
                        EventPayload::Error(e) => {
                            error!("MQTT Error: {:?}", e);
                        }
                        _ => {}
                    }
                }

                warn!("Connection closed - this shouldn't happen!");
            })
            .unwrap();

        // Outer loop: Retry subscriptions until all succeed
        loop {
            let mut all_subscribed = true;

            for mqtt_topic in mqtt_topics {
                if let Err(e) = client.subscribe(mqtt_topic.topic, mqtt_topic.qos) {
                    error!("Failed to subscribe to topic \"{}\": {:?}, retrying...", mqtt_topic.topic, e);
                    all_subscribed = false;
                    std::thread::sleep(Duration::from_millis(500));
                    break;
                }
            }

            if all_subscribed {
                for mqtt_topic in mqtt_topics {
                    info!("Subscribed to topic \"{}\"", mqtt_topic.topic);
                }
                std::thread::sleep(Duration::from_millis(500));

                // Inner loop: Keep the main thread alive
                loop {
                    std::thread::sleep(Duration::from_secs(60));
                }
            }
        }
    })
}