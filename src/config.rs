use std::collections::HashMap;

use cosmic::{
    cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry},
    iced::Subscription,
};
use serde::{Deserialize, Serialize};

use crate::{
    app::{APPID, AppMsg},
    monitor::DisplayId,
};

pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, CosmicConfigEntry, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub monitors: HashMap<DisplayId, MonitorConfig>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MonitorConfig {
    pub gamma_map: f32,
    /// Whether this monitor should respond to F1/F2 brightness keys
    #[serde(default = "default_sync_enabled")]
    pub sync_with_brightness_keys: bool,
    /// Minimum brightness percentage (0-100) that will be sent to hardware
    #[serde(default = "default_min_brightness")]
    pub min_brightness: u16,
}

fn default_sync_enabled() -> bool {
    true  // Default to enabled for all monitors
}

fn default_min_brightness() -> u16 {
    0  // Default to no minimum
}

impl MonitorConfig {
    pub fn new() -> Self {
        Self {
            gamma_map: 1.,
            sync_with_brightness_keys: true,
            min_brightness: 0,
        }
    }

    pub fn with_default_gamma(gamma: f32) -> Self {
        Self {
            gamma_map: gamma,
            sync_with_brightness_keys: true,
            min_brightness: 0,
        }
    }
}

impl Config {
    pub fn get_gamma_map(&self, id: &str) -> f32 {
        self.monitors.get(id).map(|m| m.gamma_map).unwrap_or_else(|| {
            // Default gamma based on display type
            if id.starts_with("apple-hid-") {
                // Apple displays and LG UltraFine displays (which use Apple HID protocol) work better with 1.8
                1.8
            } else {
                // DDC displays default to linear (1.0)
                1.0
            }
        })
    }

    pub fn is_sync_enabled(&self, id: &str) -> bool {
        self.monitors.get(id).map(|m| m.sync_with_brightness_keys).unwrap_or(true)
    }

    pub fn get_min_brightness(&self, id: &str) -> u16 {
        self.monitors.get(id).map(|m| m.min_brightness).unwrap_or(0)
    }
}

pub fn sub() -> Subscription<AppMsg> {
    struct ConfigSubscription;

    cosmic_config::config_subscription(
        std::any::TypeId::of::<ConfigSubscription>(),
        APPID.into(),
        CONFIG_VERSION,
    )
    .map(|update| {
        if !update.errors.is_empty() {
            error!("can't load config {:?}: {:?}", update.keys, update.errors);
        }
        AppMsg::ConfigChanged(update.config)
    })
}
