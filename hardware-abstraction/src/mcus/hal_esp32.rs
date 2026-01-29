use std::time::{Duration, Instant};
use esp_idf_hal::gpio::{PinDriver, Input, Output, AnyIOPin, AnyOutputPin, AnyInputPin};
use esp_idf_hal::ledc::LedcDriver;
use software_defined_hive::state::actuators::{HoneyCellDisplacerCommand, HoneyCellDisplacer, HoneyCellDisplacerFault};

pub struct Esp32Actuator<'actuator_lifetime> {
    pwm: LedcDriver<'actuator_lifetime>,
    dir_a: PinDriver<'actuator_lifetime, AnyOutputPin, Output>,
    dir_b: PinDriver<'actuator_lifetime, AnyOutputPin, Output>,
    limit_top: PinDriver<'actuator_lifetime, AnyInputPin, Input>,
    limit_bottom: PinDriver<'actuator_lifetime, AnyInputPin, Input>,
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
        dir_a: PinDriver<'actuator_lifetime, AnyOutputPin, Output>,
        dir_b: PinDriver<'actuator_lifetime, AnyOutputPin, Output>,
        limit_top: PinDriver<'actuator_lifetime, AnyInputPin, Input>,
        limit_bottom: PinDriver<'actuator_lifetime, AnyInputPin, Input>,
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

    fn wait_until_limit(&self, limit: &PinDriver<'actuator_lifetime, AnyInputPin, Input>) -> Result<(), HoneyCellDisplacerFault> {
        let start = Instant::now();
        while limit.is_high() {
            if start.elapsed() > self.max_move_duration {
                return Err(HoneyCellDisplacerFault::Timeout);
            }
        }
        Ok(())
    }
}