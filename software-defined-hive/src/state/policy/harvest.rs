use serde::Serialize;

/// Most of these values can be re-calibrated and delivered as Over the Air (OTA) updates

#[derive(Debug, Clone, Serialize)]
pub struct HarvestPolicyConfigs {
    /// Minimum weight to consider the batch as a harvest candidate (grams)
    pub min_honey_weight_g: u32,

    /// Weight change (give or take) over time required to consider the batch as a harvest candidate (grams)
    pub stable_delta_g: u32,

    /// Stability window in seconds - the total amount of time in which stable_delta_g and min_honey_weight_g tally with the desired values
    pub stability_window_s: u64,

    /// Maximum drain time is the time limit allowed for the Draining state of the hive to consider moving to the Closing state. N/B - I might consider using weight from the sensor because time might be affected by the viscosity of honey.
    pub max_drain_time_s: u64,
}

impl Default for HarvestPolicyConfigs {
    fn default() -> Self {
        Self {
            min_honey_weight_g: 5000,
            stable_delta_g: 50,
            stability_window_s: 300,
            max_drain_time_s: 600,
        }
    }
}
