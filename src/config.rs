use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, CosmicConfigEntry, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Config {
    pub monitors: Vec<Monitor>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            monitors: Vec::new(),
        }
    }
}
impl Config {
    pub fn get_gamma_map(&self, id: &String) -> Option<f32> {
        self.monitors
            .iter()
            .find(|x| &x.id == id)
            .and_then(|x| Some(x.gamma_map))
    }
    pub fn get_monitor(&self, id: &String) -> Option<&Monitor> {
        self.monitors.iter().find(|x| &x.id == id)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Monitor {
    pub id: String,
    pub gamma_map: f32,
}
