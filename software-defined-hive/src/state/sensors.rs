use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SensorReadings {
    /// Total calibrated net hive weight in grams
    pub weight_g: u32,
    /// Internal hive temperature (°C * 10) -  The rationale for this is to stick to int operations in the MCU so if temp is 23.5°C, we treat it as 235, our precision will ALWAYS be 1 decimal place
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_x10: Option<i16>,
    /// External hive temperature (°C * 10) - This might be important for big data; it might give more insights into making the process more precise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_temperature_x10: Option<i16>,
    /// Relative humidity (% * 10) -  The rationale for this is to stick to int operations in the MCU so if humidity is 42.5%, we treat it as 425, our precision will ALWAYS be 1 decimal place
    #[serde(skip_serializing_if = "Option::is_none")]
    pub humidity_x10: Option<u16>,
    /// Timestamp (monotonic seconds)
    pub timestamp_s: u64,
}
