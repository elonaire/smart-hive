use esp_idf_hal::gpio::*;
use esp_idf_hal::ledc::*;
use esp_idf_hal::ledc::{
    LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver, Resolution, config::TimerConfig,
};
use esp_idf_hal::prelude::*;
use esp_idf_svc::mqtt::client::{EspMqttClient as MqttClient, MqttClientConfiguration, QoS};
use esp_idf_sys as _; // ESP-IDF runtime
use hardware_abstraction::mcus::hal_esp32::Esp32Actuator;
use software_defined_hive::state::actuators::{HoneyCellDisplacer, HoneyCellDisplacerCommand};
use std::time::Duration;
use std::thread;

fn main() -> ! {
    // Initialize ESP-IDF runtime
    esp_idf_sys::link_patches();

    // Configure and connect to Wi-Fi
    let peripherals = Peripherals::take().unwrap();

    // Configure PWM / direction / limit switches
    let timer_config = TimerConfig {
        frequency: Hertz::from(5000),
        resolution: Resolution::Bits10,
    };
    let ledc_timer = LedcTimerDriver::new(peripherals.ledc.timer0, &timer_config).unwrap();

    let pwm_channel = LedcDriver::new(
        peripherals.ledc.channel0,
        &ledc_timer,
        peripherals.pins.gpio18,
    )
    .unwrap();

    let dir_a = PinDriver::output(peripherals.pins.gpio19).unwrap();
    let dir_b = PinDriver::output(peripherals.pins.gpio21).unwrap();

    let limit_top = PinDriver::input(peripherals.pins.gpio34).unwrap();
    let limit_bottom = PinDriver::input(peripherals.pins.gpio35).unwrap();

    let mut actuator = Esp32Actuator::new(
        pwm_channel,
        dir_a,
        dir_b,
        limit_top,
        limit_bottom,
        Duration::from_secs(5),
    );

    actuator.home().unwrap();

    // Configure MQTT client
    let mqtt_config = MqttClientConfiguration {
        client_id: Some("hive_actuator".into()),
        ..Default::default()
    };

    let (mut mqtt_client, mut connection) =
        MqttClient::new("mqtt://host.wokwi.internal:1883", &mqtt_config).unwrap();

    mqtt_client.subscribe("hive/actuator/command", QoS::ExactlyOnce).unwrap();
    println!("Subscribed to MQTT topic: hive/actuator/command");

    loop {
        // Check for incoming messages
        if let Ok(event) = connection.next() {
            if let Ok(payload_str) = std::str::from_utf8(event.payload().to_string().as_bytes()) {
                let command = match payload_str.trim() {
                    "SlideUp" => HoneyCellDisplacerCommand::SlideUp,
                    "SlideDown" => HoneyCellDisplacerCommand::SlideDown,
                    "Stop" => HoneyCellDisplacerCommand::Stop,
                    _ => {
                        println!("Unknown command: {}", payload_str);
                        continue;
                    }
                };

                actuator.execute(command).ok();
                println!("Executed command via MQTT: {:?}", command);
            };
        }

        thread::sleep(Duration::from_millis(100));
    }
}
