use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, CosmicConfigEntry, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Config {
    pub gamma_curves: Vec<(String, f32)>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            gamma_curves: Vec::new(),
        }
    }
}
