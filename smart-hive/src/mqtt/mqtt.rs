use std::time::Duration;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::sys::EspError;

pub fn mqtt_create(
    url: &str,
    client_id: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let (mqtt_client, mqtt_conn) = EspMqttClient::new(
        url,
        &MqttClientConfiguration {
            client_id: Some(client_id),
            username,
            password,
            keep_alive_interval: Some(Duration::from_secs(30)),
            disable_clean_session: false,
            ..Default::default()
        },
    )?;

    Ok((mqtt_client, mqtt_conn))
}