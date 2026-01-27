use crate::state::sensors::SensorReadings;
use crate::state::traits::{HumiditySensor, SensorError, TemperatureSensor, WeightSensor};

pub struct SensorDataAggregator<Weight, InternalTemperature, ExternalTemperature, Humidity> {
    weight: Weight,
    internal_temp: InternalTemperature,
    external_temp: ExternalTemperature,
    humidity: Humidity,
}

impl<Weight, InternalTemperature, ExternalTemperature, Humidity> SensorDataAggregator<Weight, InternalTemperature, ExternalTemperature, Humidity>
where
    Weight: WeightSensor,
    InternalTemperature: TemperatureSensor,
    ExternalTemperature: TemperatureSensor,
    Humidity: HumiditySensor,
{
    pub fn aggregate_sensor_readings(&mut self, timestamp_s: u64) -> Result<SensorReadings, SensorError> {
        Ok(SensorReadings {
            weight_g: self.weight.read_grams()?,
            temperature_x10: self.internal_temp.read_celsius_x10()?,
            external_temperature_x10: self.external_temp.read_celsius_x10()?,
            humidity_x10: self.humidity.read_percent_x10()?,
            timestamp_s,
        })
    }
}
