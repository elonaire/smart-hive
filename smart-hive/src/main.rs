use esp_idf_hal::gpio::*;
use esp_idf_hal::ledc::*;
use esp_idf_hal::ledc::{
    LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver, Resolution, config::TimerConfig,
};
use esp_idf_hal::prelude::*;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::mqtt::client::{EspMqttClient as MqttClient, MqttClientConfiguration};
use esp_idf_svc::netif::*;
// use esp_idf_svc::wifi::*;
use esp_idf_sys as _; // ESP-IDF runtime
use hardware_abstraction::mcus::hal_esp32::Esp32Actuator;
use software_defined_hive::state::actuators::{HoneyCellDisplacer, HoneyCellDisplacerCommand};
use std::time::Duration;
use std::{env, thread};

fn main() -> ! {
    // Initialize ESP-IDF runtime
    esp_idf_sys::link_patches();

    // Configure and connect to Wi-Fi
    let peripherals = Peripherals::take().unwrap();
    // let sysloop = EspSystemEventLoop::take().unwrap();
    // let mut wifi = EspWifi::new(peripherals.modem, sysloop.clone()).unwrap();
    // let mut wifi = EspWifi::new();

    // let ssid = env::var("WIFI_SSID").expect("WIFI_SSID not set");
    // let password = env::var("WIFI_PASS").expect("WIFI_PASS not set");
    //
    // wifi.set_configuration(&Configuration::Client(ClientConfiguration {
    //     ssid,
    //     password,
    //     ..Default::default()
    // }))
    // .unwrap();
    // wifi.start().unwrap();
    // wifi.connect().unwrap();
    //
    // println!("Wi-Fi connected!");

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

    let dir_a = peripherals.pins.gpio19.into_output().unwrap();
    let dir_b = peripherals.pins.gpio21.into_output().unwrap();

    let limit_top = peripherals.pins.gpio34.into_input().unwrap();
    let limit_bottom = peripherals.pins.gpio35.into_input().unwrap();

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

    mqtt_client.subscribe("hive/actuator/command", 1).unwrap();
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
