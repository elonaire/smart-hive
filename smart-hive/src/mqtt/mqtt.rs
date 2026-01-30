use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::sys::EspError;

pub fn mqtt_create(
    url: &str,
    client_id: &str,
    username: &str,
    password: &str,
) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let (mqtt_client, mqtt_conn) = EspMqttClient::new(
        url,
        &MqttClientConfiguration {
            client_id: Some(client_id),
            username: Some(username),
            password: Some(password),
            ..Default::default()
        },
    )?;

    Ok((mqtt_client, mqtt_conn))
}