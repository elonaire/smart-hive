use serde::{Deserialize, Serialize};
use log::info;

use crate::state::policy::harvest::HarvestPolicyConfigs;
use crate::state::actuators::{HoneyCellDisplacer, HoneyCellDisplacerCommand};
use crate::state::hive::HiveState;
use crate::state::sensors::SensorReadings;

/// This describes the brain of the hive
pub struct HiveController<H: HoneyCellDisplacer> {
    state: HiveState,
    policy: HarvestPolicyConfigs,
    honey_cell_displacer: H,

    // Internal memory
    last_weight_g: Option<u32>,
    stable_since: Option<u64>,
    drain_started_at: Option<u64>,

    // Latched intent
    authorized: bool,
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
            authorized: false,
        }
    }

    // SENSOR UPDATE (DRIVES FSM)

    pub fn update(&mut self, reading: SensorReadings) {
        match self.state {
            HiveState::Monitoring => {
                self.last_weight_g = Some(reading.weight_g);

                if reading.weight_g >= self.policy.min_honey_weight_g {
                    self.state = HiveState::Candidate;
                    self.stable_since = None;
                }
            }

            HiveState::Candidate => {
                if let Some(last) = self.last_weight_g {
                    let delta = last.abs_diff(reading.weight_g);

                    if delta <= self.policy.stable_delta_g {
                        self.stable_since
                            .get_or_insert(reading.timestamp_s);

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
                if self.authorized {
                    self.authorized = false;
                    self.enter_authorized(reading.timestamp_s);
                }
            }

            HiveState::Draining => {
                if reading.timestamp_s
                    - self.drain_started_at.unwrap()
                    >= self.policy.max_drain_time_s
                {
                    self.enter_closing();
                }
            }

            HiveState::Verifying => {
                if let Some(last) = self.last_weight_g {
                    if reading.weight_g < last {
                        self.reset_to_monitoring();
                    }
                }
            }

            HiveState::Fault => {
                // Locked until ResetFault
            }

            _ => {}
        }

        self.last_weight_g = Some(reading.weight_g);
    }

    // COMMAND HANDLING (INTENT)

    pub fn process_command(&mut self, command: HiveCommand) -> Result<Option<String>, String> {
        match command {
            HiveCommand::AuthorizeHarvest => {
                if self.state == HiveState::Ready {
                    self.authorized = true;
                    info!("Harvest authorized");
                    Ok(None)
                } else {
                    Err(format!("Cannot authorize harvest in state {:?}", self.state))
                }
            }

            HiveCommand::CancelHarvest => {
                if matches!(self.state, HiveState::Ready | HiveState::Draining) {
                    self.reset_to_monitoring();
                    Ok(None)
                } else {
                    Err(format!("Cannot cancel harvest in state {:?}", self.state))
                }
            }

            HiveCommand::EmergencyStop => {
                let _ = self.honey_cell_displacer
                    .execute(HoneyCellDisplacerCommand::Stop);
                self.state = HiveState::Fault;
                Ok(None)
            }

            HiveCommand::ResetFault => {
                if self.state == HiveState::Fault {
                    self.reset_to_monitoring();
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

                Ok(Some(
                    serde_json::to_string(&PolicyUpdateResponse {
                        status: "success".into(),
                        policy,
                    }).map_err(|e| e.to_string())?
                ))
            }

            HiveCommand::GetPolicy => {
                Ok(Some(serde_json::to_string(&self.policy).unwrap()))
            }

            HiveCommand::GetStatus => {
                Ok(Some(serde_json::to_string(&self.get_status()).unwrap()))
            }
        }
    }


    // STATE ENTRY ACTIONS

    fn enter_authorized(&mut self, now: u64) {
        match self.honey_cell_displacer.execute(HoneyCellDisplacerCommand::SlideDown) {
            Ok(_) => {
                self.drain_started_at = Some(now);
                self.state = HiveState::Draining;
            }
            Err(_) => self.state = HiveState::Fault,
        }
    }

    fn enter_closing(&mut self) {
        match self.honey_cell_displacer.execute(HoneyCellDisplacerCommand::SlideUp) {
            Ok(_) => self.state = HiveState::Verifying,
            Err(_) => self.state = HiveState::Fault,
        }
    }

    fn reset_to_monitoring(&mut self) {
        self.state = HiveState::Monitoring;
        self.authorized = false;
        self.stable_since = None;
        self.drain_started_at = None;
    }


    // STATUS / POLICY

    pub fn state(&self) -> HiveState {
        self.state
    }

    pub fn get_status(&self) -> HiveStatus {
        HiveStatus {
            state: self.state,
            last_weight_g: self.last_weight_g,
            stable_since: self.stable_since,
            drain_started_at: self.drain_started_at,
            policy: self.policy.clone(),
        }
    }

    fn validate_policy(&self, policy: &HarvestPolicyConfigs) -> Result<(), String> {
        if policy.min_honey_weight_g == 0
            || policy.stable_delta_g == 0
            || policy.stability_window_s == 0
            || policy.max_drain_time_s == 0
            || policy.max_drain_time_s > 3600
        {
            return Err("Invalid policy configuration".into());
        }
        Ok(())
    }
}
