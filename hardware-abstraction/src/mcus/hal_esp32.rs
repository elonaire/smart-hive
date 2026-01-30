use std::time::{Duration, Instant};
use esp_idf_hal::gpio::{PinDriver, Input, Output, Gpio19, Gpio21, Gpio34, Gpio35};
use esp_idf_hal::ledc::LedcDriver;
use software_defined_hive::state::actuators::{HoneyCellDisplacerCommand, HoneyCellDisplacer, HoneyCellDisplacerFault};

/// This describes the actuator(honey cell displacer) in terms of software
///
/// `pwm` controls motor speed i.e. the speed in which the honey cell displacers will move up and down
///
/// `dir_a` is a direction in which the motor moves
///
/// `dir_b` is the alternate direction i.e. when `dir_b = 0` if and only if `dir_a = 1` and vice versa
///
/// `limit_top` and `limit_bottom` define physical bounds of honey cell displacers - very essential for homing
///
/// `max_move_duration` time limit for the traveling of honey cell displacers
pub struct Esp32Actuator<'actuator_lifetime> {
    pwm: LedcDriver<'actuator_lifetime>,
    dir_a: PinDriver<'actuator_lifetime, Gpio19, Output>,
    dir_b: PinDriver<'actuator_lifetime, Gpio21, Output>,
    limit_top: PinDriver<'actuator_lifetime, Gpio34, Input>,
    limit_bottom: PinDriver<'actuator_lifetime, Gpio35, Input>,
    max_move_duration: Duration,
}

impl HoneyCellDisplacer for Esp32Actuator<'_> {
    fn execute(&mut self, cmd: HoneyCellDisplacerCommand) -> Result<(), HoneyCellDisplacerFault> {
        match cmd {
            HoneyCellDisplacerCommand::SlideUp => self.slide_up(),
            HoneyCellDisplacerCommand::SlideDown => self.slide_down(),
            HoneyCellDisplacerCommand::Stop => self.stop(),
        }
    }
}

impl<'actuator_lifetime> Esp32Actuator<'actuator_lifetime> {
    pub fn new(
        pwm: LedcDriver<'actuator_lifetime>,
        dir_a: PinDriver<'actuator_lifetime, Gpio19, Output>,
        dir_b: PinDriver<'actuator_lifetime, Gpio21, Output>,
        limit_top: PinDriver<'actuator_lifetime, Gpio34, Input>,
        limit_bottom: PinDriver<'actuator_lifetime, Gpio35, Input>,
        max_move_duration: Duration,
    ) -> Self {
        Self {
            pwm,
            dir_a,
            dir_b,
            limit_top,
            limit_bottom,
            max_move_duration
        }
    }

    fn slide_up(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        self.set_direction_up()?;
        self.enable_motion()?;
        self.wait_until_limit(&self.limit_top)?;
        self.stop()?;
        Ok(())
    }

    fn slide_down(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        if self.limit_bottom.is_low() {
            return Err(HoneyCellDisplacerFault::EndStopHit);
        }

        self.dir_a.set_low().map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        self.dir_b.set_high().map_err(|_| HoneyCellDisplacerFault::Hardware)?;

        Ok(())
    }

    fn stop(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        self.disable_motion();
        Ok(())
    }

    pub fn home(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        self.slide_down()?;
        Ok(())
    }

    fn enable_motion(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        self.pwm.set_duty(self.pwm.get_max_duty() / 2).map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        Ok(())
    }

    fn disable_motion(&mut self) {
        let _ = self.pwm.set_duty(0);
    }

    fn set_direction_up(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        if self.limit_top.is_low() {
            return Err(HoneyCellDisplacerFault::EndStopHit);
        }
        self.dir_a.set_high().map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        self.dir_b.set_low().map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        Ok(())
    }

    fn set_direction_down(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        if self.limit_bottom.is_low() {
            return Err(HoneyCellDisplacerFault::EndStopHit);
        }
        self.dir_a.set_low().map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        self.dir_b.set_high().map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        Ok(())
    }

    fn wait_until_limit(&self, limit: &PinDriver<'actuator_lifetime, Gpio34, Input>) -> Result<(), HoneyCellDisplacerFault> {
        let start = Instant::now();
        while limit.is_high() {
            if start.elapsed() > self.max_move_duration {
                return Err(HoneyCellDisplacerFault::Timeout);
            }
        }
        Ok(())
    }
}