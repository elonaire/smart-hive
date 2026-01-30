use serde::{Deserialize, Serialize};

/// The lifecycle of the hive from one harvest to another
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HiveState {
    Monitoring,
    Candidate,
    Ready,
    Authorized,
    Actuating,
    Draining,
    Closing,
    Verifying,
    Fault,
}
