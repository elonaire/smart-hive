use crate::state::policy::harvest::HarvestPolicyConfigs;
use crate::state::actuators::{HoneyCellDisplacer, HoneyCellDisplacerCommand};
use crate::state::hive::HiveState;
use crate::state::sensors::SensorReadings;

pub struct HiveController<H: HoneyCellDisplacer> {
    state: HiveState,
    policy: HarvestPolicyConfigs,
    honey_cell_displacer: H,

    // Internal memory
    last_weight_g: Option<u32>,
    stable_since: Option<u64>,
    drain_started_at: Option<u64>,
}

impl<H: HoneyCellDisplacer> HiveController<H> {
    pub fn new(policy: HarvestPolicyConfigs, honey_cell_displacer: H) -> Self {
        Self {
            state: HiveState::Monitoring,
            policy,
            honey_cell_displacer,
            last_weight_g: None,
            stable_since: None,
            drain_started_at: None,
        }
    }

    pub fn update(&mut self, reading: SensorReadings, authorized: bool) {
        match self.state {
            HiveState::Monitoring => {
                if reading.weight_g >= self.policy.min_honey_weight_g {
                    self.state = HiveState::Candidate;
                }
            }

            HiveState::Candidate => {
                if let Some(last) = self.last_weight_g {
                    let delta = last.abs_diff(reading.weight_g);

                    if delta <= self.policy.stable_delta_g {
                        self.stable_since.get_or_insert(reading.timestamp_s);

                        if reading.timestamp_s
                            - self.stable_since.unwrap()
                            >= self.policy.stability_window_s
                        {
                            self.state = HiveState::Ready;
                        }
                    } else {
                        self.stable_since = None;
                    }
                }
            }

            HiveState::Ready => {
                if authorized {
                    self.state = HiveState::Authorized;
                }
            }

            HiveState::Authorized => {
                if self.honey_cell_displacer.execute(HoneyCellDisplacerCommand::SlideDown).is_ok() {
                    self.drain_started_at = Some(reading.timestamp_s);
                    self.state = HiveState::Draining;
                } else {
                    self.state = HiveState::Fault;
                }
            }

            HiveState::Draining => {
                if reading.timestamp_s - self.drain_started_at.unwrap()
                    >= self.policy.max_drain_time_s
                {
                    self.state = HiveState::Closing;
                }
            }

            HiveState::Closing => {
                if self.honey_cell_displacer.execute(HoneyCellDisplacerCommand::SlideUp).is_ok() {
                    self.state = HiveState::Verifying;
                } else {
                    self.state = HiveState::Fault;
                }
            }

            HiveState::Verifying => {
                // Weight reduction confirms harvest
                if let Some(last) = self.last_weight_g {
                    if reading.weight_g < last {
                        self.state = HiveState::Monitoring;
                    }
                }
            }

            HiveState::Fault => {
                // Require manual reset
            }

            _ => {}
        }

        self.last_weight_g = Some(reading.weight_g);
    }

    pub fn state(&self) -> HiveState {
        self.state
    }
}

