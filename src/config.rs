use std::collections::HashMap;

use cosmic::{
    cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry},
    iced::Subscription,
};
use serde::{Deserialize, Serialize};

use crate::{
    app::{APPID, AppMessage},
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
}

impl MonitorConfig {
    pub fn new() -> Self {
        Self { gamma_map: 1. }
    }
}

impl Config {
    pub fn get_gamma_map(&self, id: &str) -> f32 {
        self.monitors.get(id).map(|m| m.gamma_map).unwrap_or(1.)
    }
}

pub fn sub() -> Subscription<AppMessage> {
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
        AppMessage::ConfigChanged(update.config)
    })
}
