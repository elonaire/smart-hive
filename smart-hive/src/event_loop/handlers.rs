use std::sync::{Arc, Mutex};
use esp_idf_svc::mqtt::client::{EspMqttClient, QoS};
use log::*;
use serde::Serialize;
use software_defined_hive::controller::controller::{HiveCommand, HiveController};
use software_defined_hive::state::actuators::HoneyCellDisplacer;
use software_defined_hive::state::hive::HiveState;
use software_defined_hive::state::sensors::SensorReadings;

/// Handler for all hive commands received as MQTT messages
/// qos is the quality of service (QoS)
pub fn handle_command<H: HoneyCellDisplacer>(
    payload: &str,
    controller: &Arc<Mutex<HiveController<H>>>,
    client: &Arc<Mutex<EspMqttClient<'_>>>,
    qos: &QoS,
) {
    match serde_json::from_str::<HiveCommand>(payload) {
        Ok(command) => {
            info!("Received command: {:?}", command);

            let mut ctrl = controller.lock().unwrap();
            match ctrl.process_command(command) {
                Ok(response) => {
                    info!("Command processed successfully. New state: {:?}", ctrl.state());

                    if let Some(resp) = response {
                        publish_message(client, "smart-hive/responses", &resp, qos);
                    }
                }
                Err(e) => {
                    error!("Command failed: {}", e);

                    let error_response = format!(r#"{{"status":"error","message":"{}"}}"#, e);
                    publish_message(client, "smart-hive/responses", &error_response, &QoS::AtLeastOnce); // AtLeastOnce because duplicates won't hurt - it's just an error message
                }
            }
        }
        Err(e) => {
            error!("Failed to parse command: {}", e);
        }
    }
}

/// Handler for sensor readings
/// qos the quality of service (QoS)
pub fn handle_sensor_reading<H: HoneyCellDisplacer>(
    payload: &str,
    controller: &Arc<Mutex<HiveController<H>>>,
    client: &Arc<Mutex<EspMqttClient<'_>>>,
    qos: &QoS,
) {
    match serde_json::from_str::<SensorReadings>(payload) {
        Ok(reading) => {
            let mut ctrl = controller.lock().unwrap();
            let previous_state = ctrl.state();

            // Update controller with sensor reading
            ctrl.update(reading, false);

            let new_state = ctrl.state();

            // Notify on state changes
            if previous_state != new_state {
                info!("State transition: {:?} -> {:?}", previous_state, new_state);

                // Publish state change notification
                let notification = StateChangeNotification {
                    previous_state,
                    new_state,
                    weight_g: reading.weight_g,
                    timestamp_s: reading.timestamp_s,
                };

                if let Ok(json) = serde_json::to_string(&notification) {
                    publish_message(client, "smart-hive/notifications/state-change", &json, qos);
                }

                // Special notification when harvest is ready
                if new_state == HiveState::Ready {
                    info!("The Hive is harvest-ready! Net Weight: {}g", reading.weight_g);

                    let ready_notification = HarvestReadyNotification {
                        message: "Harvest is ready for authorization".to_string(),
                        weight_g: reading.weight_g,
                        timestamp_s: reading.timestamp_s,
                    };

                    if let Ok(json) = serde_json::to_string(&ready_notification) {
                        publish_message(client, "smart-hive/notifications/harvest-ready", &json, &QoS::AtLeastOnce); // AtLeastOnce because duplicates won't hurt - it's just an error message
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to parse sensor reading: {}", e);
        }
    }
}

/// Helper function to publish messages (this logic is repetitive)
/// qos is the quality of service (QoS)
fn publish_message(
    client: &Arc<Mutex<EspMqttClient<'_>>>,
    topic: &str,
    message: &str,
    qos: &QoS,
) {
    let mut mqtt_client = client.lock().unwrap();
    if let Err(e) = mqtt_client.enqueue(
        topic,
        qos.to_owned(),
        false,
        message.as_bytes()
    ) {
        error!("Failed to publish to {}: {}", topic, e);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StateChangeNotification {
    pub previous_state: HiveState,
    pub new_state: HiveState,
    pub weight_g: u32,
    pub timestamp_s: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarvestReadyNotification {
    pub message: String,
    pub weight_g: u32,
    pub timestamp_s: u64,
}