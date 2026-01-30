mod mqtt;
mod wi_fi;
mod event_loop;

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
use crate::event_loop::event_loop::create_event_loop;
use crate::mqtt::mqtt::mqtt_create;
use crate::wi_fi::wi_fi::wifi_create;
use log::*;

const MQTT_BROKER_URL: &str = env!("MQTT_BROKER_URL");
const MQTT_CLIENT_ID: &str = env!("MQTT_CLIENT_ID");

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

    esp_idf_svc::log::EspLogger::initialize_default();

    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let _wifi = wifi_create(&sys_loop, &nvs, modem).unwrap();

    let (mut client, mut conn) = mqtt_create("mqtt://host.wokwi.internal:1883", "smart-hive").unwrap();

    create_event_loop(&mut client, &mut conn, "smart-hive/start-harvest").unwrap();
}
