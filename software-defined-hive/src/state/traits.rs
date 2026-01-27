use std::io::Error;

// This might need to be revised
pub type SensorError = Error;

pub trait WeightSensor {
    fn read_grams(&mut self) -> Result<u32, SensorError>;
}

pub trait TemperatureSensor {
    fn read_celsius_x10(&mut self) -> Result<i16, SensorError>;
}

pub trait HumiditySensor {
    fn read_percent_x10(&mut self) -> Result<u16, SensorError>;
}
