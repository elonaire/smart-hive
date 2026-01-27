use std::time::{Duration, Instant};
use esp_idf_hal::gpio::{AnyInputPin, AnyOutputPin};
use esp_idf_hal::ledc::LedcDriver;
use software_defined_hive::state::actuators::{HoneyCellDisplacerCommand, HoneyCellDisplacer, HoneyCellDisplacerFault};

pub struct Esp32Actuator<'actuator_lifetime> {
    pwm: LedcDriver<'actuator_lifetime>,
    dir_a: AnyOutputPin,
    dir_b: AnyOutputPin,
    limit_top: AnyInputPin,
    limit_bottom: AnyInputPin,
    max_move_duration: Duration, // safety timeout
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
        dir_a: AnyOutputPin,
        dir_b: AnyOutputPin,
        limit_top: AnyInputPin,
        limit_bottom: AnyInputPin,
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
        self.wait_until_limit(self.limit_top)?;
        self.stop()?;
        Ok(())
    }

    fn slide_down(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        if self.limit_top.is_low() {
            return Err(HoneyCellDisplacerFault::EndStopHit);
        }

        self.dir_a.set_low().ok();
        self.dir_b.set_high().ok();

        Ok(())
    }

    fn stop(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        self.disable_motion();
        Ok(())
    }

    /// Homing routine: moves down until bottom switch is hit
    pub fn home(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        self.slide_down()?;
        // after hitting bottom switch, actuator is at known zero
        Ok(())
    }

    fn enable_motion(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        self.pwm.set_duty(self.pwm.get_max_duty() / 2).map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        Ok(())
    }

    fn disable_motion(&mut self) {
        let _ = self.pwm.set_duty(0);
    }

    /// Set motor direction to up
    fn set_direction_up(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        if self.limit_top.is_low() {
            return Err(HoneyCellDisplacerFault::EndStopHit);
        }
        self.dir_a.set_high().map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        self.dir_b.set_low().map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        Ok(())
    }

    /// Set motor direction to down
    fn set_direction_down(&mut self) -> Result<(), HoneyCellDisplacerFault> {
        if self.limit_bottom.is_low() {
            return Err(HoneyCellDisplacerFault::EndStopHit);
        }
        self.dir_a.set_low().map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        self.dir_b.set_high().map_err(|_| HoneyCellDisplacerFault::Hardware)?;
        Ok(())
    }

    /// Wait until a limit switch triggers or timeout occurs
    fn wait_until_limit(&self, limit: AnyInputPin) -> Result<(), HoneyCellDisplacerFault> {
        let start = Instant::now();
        while limit.is_high().unwrap_or(false) {
            if start.elapsed() > self.max_move_duration {
                return Err(HoneyCellDisplacerFault::Timeout);
            }
        }
        Ok(())
    }
}

