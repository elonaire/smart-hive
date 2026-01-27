/// These commands are what controls the actuators that displace the honey cells during harvesting
#[derive(Debug, Clone, Copy)]
pub enum HoneyCellDisplacerCommand {
    SlideDown,
    SlideUp,
    Stop,
}

pub trait HoneyCellDisplacer {
    /// gives instruction to the actuator (honey cell displacer) to execute a command
    fn execute(&mut self, cmd: HoneyCellDisplacerCommand) -> Result<(), HoneyCellDisplacerFault>;
}

#[derive(Debug)]
pub enum HoneyCellDisplacerFault {
    OverCurrent,
    EndStopHit,
    Timeout,
    Hardware,
}
