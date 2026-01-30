use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "command")]
pub enum HiveCommand {
    #[serde(rename = "authorize_harvest")]
    AuthorizeHarvest,

    #[serde(rename = "cancel_harvest")]
    CancelHarvest,

    #[serde(rename = "emergency_stop")]
    EmergencyStop,

    #[serde(rename = "reset_fault")]
    ResetFault,

    #[serde(rename = "manual_slide_down")]
    ManualSlideDown,

    #[serde(rename = "manual_slide_up")]
    ManualSlideUp,

    #[serde(rename = "update_policy")]
    UpdatePolicy {
        policy: HarvestPolicyConfigs,
    },

    #[serde(rename = "get_policy")]
    GetPolicy,

    #[serde(rename = "get_status")]
    GetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveStatus {
    pub state: HiveState,
    pub last_weight_g: Option<u32>,
    pub stable_since: Option<u64>,
    pub drain_started_at: Option<u64>,
    pub policy: HarvestPolicyConfigs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyUpdateResponse {
    pub status: String,
    pub policy: HarvestPolicyConfigs,
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

    /// Process a command received via MQTT or other interface
    pub fn process_command(&mut self, command: HiveCommand) -> Result<Option<String>, String> {
        match command {
            HiveCommand::AuthorizeHarvest => {
                if self.state == HiveState::Ready {
                    self.state = HiveState::Authorized;
                    Ok(None)
                } else {
                    Err(format!("Cannot authorize harvest in state {:?}", self.state))
                }
            }

            HiveCommand::CancelHarvest => {
                if matches!(self.state, HiveState::Ready | HiveState::Authorized) {
                    self.state = HiveState::Monitoring;
                    self.reset_internal_state();
                    Ok(None)
                } else {
                    Err(format!("Cannot cancel harvest in state {:?}", self.state))
                }
            }

            HiveCommand::EmergencyStop => {
                let _ = self.honey_cell_displacer.execute(HoneyCellDisplacerCommand::Stop);
                self.state = HiveState::Fault;
                Ok(None)
            }

            HiveCommand::ResetFault => {
                if self.state == HiveState::Fault {
                    self.state = HiveState::Monitoring;
                    self.reset_internal_state();
                    Ok(None)
                } else {
                    Err(format!("Not in fault state, current state: {:?}", self.state))
                }
            }

            HiveCommand::ManualSlideDown => {
                self.honey_cell_displacer
                    .execute(HoneyCellDisplacerCommand::SlideDown)
                    .map_err(|e| format!("Failed to slide down: {:?}", e))?;
                Ok(None)
            }

            HiveCommand::ManualSlideUp => {
                self.honey_cell_displacer
                    .execute(HoneyCellDisplacerCommand::SlideUp)
                    .map_err(|e| format!("Failed to slide up: {:?}", e))?;
                Ok(None)
            }

            HiveCommand::UpdatePolicy { policy } => {
                self.validate_policy(&policy)?;
                self.policy = policy.clone();

                // Return confirmation with new policy
                let response = serde_json::to_string(&PolicyUpdateResponse {
                    status: "success".to_string(),
                    policy,
                }).map_err(|e| format!("Failed to serialize response: {}", e))?;

                Ok(Some(response))
            }

            HiveCommand::GetPolicy => {
                let response = serde_json::to_string(&self.policy)
                    .map_err(|e| format!("Failed to serialize policy: {}", e))?;
                Ok(Some(response))
            }

            HiveCommand::GetStatus => {
                let status = self.get_status();
                let response = serde_json::to_string(&status)
                    .map_err(|e| format!("Failed to serialize status: {}", e))?;
                Ok(Some(response))
            }
        }
    }

    /// Validate policy configurations before applying
    fn validate_policy(&self, policy: &HarvestPolicyConfigs) -> Result<(), String> {
        if policy.min_honey_weight_g == 0 {
            return Err("min_honey_weight_g must be greater than 0".to_string());
        }
        if policy.stable_delta_g == 0 {
            return Err("stable_delta_g must be greater than 0".to_string());
        }
        if policy.stability_window_s == 0 {
            return Err("stability_window_s must be greater than 0".to_string());
        }
        if policy.max_drain_time_s == 0 {
            return Err("max_drain_time_s must be greater than 0".to_string());
        }
        if policy.max_drain_time_s > 3600 {
            return Err("max_drain_time_s cannot exceed 1 hour (3600s)".to_string());
        }
        Ok(())
    }

    /// Get current policy
    pub fn get_policy(&self) -> &HarvestPolicyConfigs {
        &self.policy
    }

    /// Get current state for status reporting
    pub fn get_status(&self) -> HiveStatus {
        HiveStatus {
            state: self.state,
            last_weight_g: self.last_weight_g,
            stable_since: self.stable_since,
            drain_started_at: self.drain_started_at,
            policy: self.policy.clone(),
        }
    }

    fn reset_internal_state(&mut self) {
        self.stable_since = None;
        self.drain_started_at = None;
    }
}

