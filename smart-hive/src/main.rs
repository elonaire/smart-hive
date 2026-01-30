mod mqtt;
mod wi_fi;
mod event_loop;

use std::sync::{Arc, Mutex};
use esp_idf_hal::gpio::*;
use esp_idf_hal::ledc::*;
use esp_idf_hal::ledc::{
    LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver, Resolution, config::TimerConfig,
};
use esp_idf_hal::prelude::*;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use hardware_abstraction::mcus::hal_esp32::Esp32Actuator;
use software_defined_hive::state::actuators::{HoneyCellDisplacer, HoneyCellDisplacerCommand};
use std::time::Duration;
use std::thread;
use esp_idf_svc::mqtt::client::QoS;
use crate::event_loop::event_loop::create_event_loop;
use crate::mqtt::mqtt::mqtt_create;
use crate::wi_fi::wi_fi::wifi_create;
use log::*;
use software_defined_hive::controller::controller::{HiveCommand, HiveController};
use software_defined_hive::state::policy::harvest::HarvestPolicyConfigs;

const MQTT_BROKER_URL: &str = env!("MQTT_BROKER_URL");
const MQTT_CLIENT_ID: &str = env!("MQTT_CLIENT_ID");
const MQTT_USERNAME: &str = env!("MQTT_USERNAME");
const MQTT_PASSWORD: &str = env!("MQTT_PASSWORD");

fn main() {
    // Initialize ESP-IDF runtime
    esp_idf_svc::sys::link_patches();

    // Configure and connect to Wi-Fi
    let Peripherals {
        pins,
        ledc,
        modem,
        ..
    } = Peripherals::take().unwrap();

    // Configure PWM / direction / limit switches
    let timer_config = TimerConfig {
        frequency: Hertz::from(5000),
        resolution: Resolution::Bits10,
    };
    let ledc_timer = LedcTimerDriver::new(ledc.timer0, &timer_config).unwrap();

    let pwm_channel = LedcDriver::new(
        ledc.channel0,
        &ledc_timer,
        pins.gpio18,
    )
    .unwrap();

    let dir_a = PinDriver::output(pins.gpio19).unwrap();
    let dir_b = PinDriver::output(pins.gpio21).unwrap();

    let limit_top = PinDriver::input(pins.gpio34).unwrap();
    let limit_bottom = PinDriver::input(pins.gpio35).unwrap();

    let mut actuator = Esp32Actuator::new(
        pwm_channel,
        dir_a,
        dir_b,
        limit_top,
        limit_bottom,
        Duration::from_secs(5),
    );

    if let Err(e) = actuator.home() {
        info!("Homing failed: {:?}", e);
    }

    // Create HiveController with default or loaded policy
    let policy = HarvestPolicyConfigs::default();

    let controller = Arc::new(Mutex::new(
        HiveController::new(policy, actuator)
    ));

    esp_idf_svc::log::EspLogger::initialize_default();

    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let _wifi = wifi_create(&sys_loop, &nvs, modem).unwrap();

    let (mut client, mut conn) = mqtt_create(MQTT_BROKER_URL, MQTT_CLIENT_ID, None, None).unwrap();

    // Clone for the closure
    let controller_clone = Arc::clone(&controller);

    // Clone client for publishing responses (need to wrap in Arc<Mutex> for thread safety)
    let client = Arc::new(Mutex::new(client));
    let client_clone = Arc::clone(&client);

    // Create event loop with message handler
    let mut mqtt_client = client.lock().unwrap();

    create_event_loop(
        &mut mqtt_client,
        &mut conn,
        "smart-hive/commands",
        move |payload| {
            // Try to parse as JSON command
            match serde_json::from_str::<HiveCommand>(payload) {
                Ok(command) => {
                    info!("Received command: {:?}", command);

                    let mut ctrl = controller_clone.lock().unwrap();
                    match ctrl.process_command(command) {
                        Ok(response) => {
                            info!("Command processed successfully. New state: {:?}", ctrl.state());

                            // Publish response if there is one
                            if let Some(resp) = response {
                                let mut client = client_clone.lock().unwrap();
                                if let Err(e) = client.enqueue(
                                    "smart-hive/responses",
                                    QoS::AtLeastOnce,
                                    false,
                                    resp.as_bytes()
                                ) {
                                    error!("Failed to publish response: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Command failed: {}", e);

                            // Publish error response
                            let error_response = format!(r#"{{"status":"error","message":"{}"}}"#, e);
                            let mut client = client_clone.lock().unwrap();
                            if let Err(e) = client.enqueue(
                                "smart-hive/responses",
                                QoS::AtLeastOnce,
                                false,
                                error_response.as_bytes()
                            ) {
                                error!("Failed to publish error response: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse command: {}", e);
                }
            }
        },
    ).unwrap();
}
