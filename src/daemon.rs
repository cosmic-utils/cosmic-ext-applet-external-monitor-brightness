// SPDX-License-Identifier: GPL-3.0-only
//! Brightness synchronization daemon
//!
//! This daemon listens to COSMIC's DisplayBrightness changes (F1/F2 keys) and
//! applies them to external displays based on per-monitor sync configuration.
//!
//! Supports:
//! - DDC/CI displays (standard monitors via I2C)
//! - Apple HID displays (Apple Studio Display, Pro Display XDR, LG UltraFine)
//!
//! Only activates when external displays are detected.

#[cfg(feature = "brightness-sync-daemon")]
use anyhow::{Context, Result};
#[cfg(feature = "brightness-sync-daemon")]
use std::sync::Arc;
#[cfg(feature = "brightness-sync-daemon")]
use tokio::sync::Mutex;
#[cfg(feature = "brightness-sync-daemon")]
use zbus::{proxy, Connection};
#[cfg(feature = "brightness-sync-daemon")]
use cosmic::cosmic_config::{Config as CosmicConfig, CosmicConfigEntry};

#[cfg(feature = "brightness-sync-daemon")]
use crate::protocols::DisplayProtocol;
#[cfg(feature = "brightness-sync-daemon")]
use crate::protocols::ddc_ci::DdcCiDisplay;
#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
use crate::protocols::apple_hid::AppleHidDisplay;
#[cfg(feature = "brightness-sync-daemon")]
use crate::config::{Config, CONFIG_VERSION};
#[cfg(feature = "brightness-sync-daemon")]
use crate::app::APPID;

#[cfg(feature = "brightness-sync-daemon")]
/// COSMIC Settings Daemon D-Bus proxy
#[proxy(
    interface = "com.system76.CosmicSettingsDaemon",
    default_service = "com.system76.CosmicSettingsDaemon",
    default_path = "/com/system76/CosmicSettingsDaemon"
)]
trait CosmicSettingsDaemon {
    /// DisplayBrightness property
    #[zbus(property)]
    fn display_brightness(&self) -> zbus::Result<i32>;

    /// MaxDisplayBrightness property
    #[zbus(property)]
    fn max_display_brightness(&self) -> zbus::Result<i32>;
}

#[cfg(feature = "brightness-sync-daemon")]
pub struct BrightnessSyncDaemon {
    displays: Arc<Mutex<Vec<(String, Box<dyn DisplayProtocol>)>>>,  // (id, display) pairs
    config_handler: CosmicConfig,
}

#[cfg(feature = "brightness-sync-daemon")]
impl BrightnessSyncDaemon {
    /// Create a new brightness sync daemon
    /// Returns None if no external displays are detected
    pub async fn new() -> Result<Option<Self>> {
        let mut displays: Vec<(String, Box<dyn DisplayProtocol>)> = Vec::new();

        // Enumerate DDC/CI displays
        let ddc_displays = DdcCiDisplay::enumerate();
        tracing::info!("Found {} DDC/CI display(s)", ddc_displays.len());
        for display in ddc_displays {
            let id = display.id();
            displays.push((id, Box::new(display)));
        }

        // Enumerate Apple HID displays if the feature is enabled
        #[cfg(feature = "apple-hid-displays")]
        {
            let api = hidapi::HidApi::new().context("Failed to initialize HID API")?;
            let apple_displays = AppleHidDisplay::enumerate(&api)
                .context("Failed to enumerate Apple HID displays")?;
            tracing::info!("Found {} Apple HID display(s)", apple_displays.len());
            for display in apple_displays {
                let id = display.id();
                displays.push((id, Box::new(display)));
            }
        }

        if displays.is_empty() {
            tracing::info!("No external displays detected, brightness sync daemon disabled");
            return Ok(None);
        }

        // Load configuration handler for runtime config access
        let config_handler = match CosmicConfig::new(APPID, CONFIG_VERSION) {
            Ok(handler) => {
                tracing::info!("Loaded config for per-monitor F1/F2 sync settings");
                handler
            }
            Err(err) => {
                tracing::warn!("Failed to load config: {}, monitors will default to sync enabled", err);
                CosmicConfig::new(APPID, CONFIG_VERSION).unwrap_or_else(|e| {
                    panic!("Cannot create config handler: {}", e);
                })
            }
        };

        tracing::info!(
            "Found {} external display(s), enabling brightness sync daemon with per-monitor control",
            displays.len()
        );

        Ok(Some(Self {
            displays: Arc::new(Mutex::new(displays)),
            config_handler,
        }))
    }

    pub async fn run(self) -> Result<()> {
        tracing::info!("Starting brightness sync daemon");

        // Connect to session bus
        let connection = Connection::session()
            .await
            .context("Failed to connect to D-Bus session bus")?;

        // Create proxy to COSMIC Settings Daemon
        let proxy = CosmicSettingsDaemonProxy::new(&connection)
            .await
            .context("Failed to create COSMIC Settings Daemon proxy")?;

        tracing::info!("Connected to COSMIC Settings Daemon");

        // Get max brightness for conversion
        let max_brightness = proxy
            .max_display_brightness()
            .await
            .context("Failed to get max display brightness")?;

        tracing::info!("Max display brightness: {}", max_brightness);

        // Subscribe to DisplayBrightness property changes
        use futures::StreamExt;
        let mut brightness_changed = proxy.receive_display_brightness_changed().await;

        tracing::info!("Listening for COSMIC brightness changes (F1/F2 keys)...");

        while let Some(change) = brightness_changed.next().await {
            if let Ok(brightness) = change.get().await {
                tracing::debug!("COSMIC brightness changed to: {}", brightness);

                // Convert COSMIC brightness (0-max) to percentage (0-100)
                let percentage = if max_brightness > 0 {
                    ((brightness as f64 / max_brightness as f64) * 100.0) as u16
                } else {
                    0
                };
                let percentage = percentage.min(100);

                tracing::info!(
                    "Applying {}% to external displays (COSMIC value: {}/{})",
                    percentage,
                    brightness,
                    max_brightness
                );

                // Apply brightness based on per-monitor sync configuration
                let config = match Config::get_entry(&self.config_handler) {
                    Ok(config) => config,
                    Err((errs, config)) => {
                        tracing::warn!("Errors loading config: {:?}, using defaults", errs);
                        config
                    }
                };

                let slider_value = percentage as f32 / 100.0;
                let mut displays = self.displays.lock().await;
                let mut synced_count = 0;
                for (id, display) in displays.iter_mut() {
                    if config.is_sync_enabled(id) {
                        // Apply gamma correction for this monitor
                        let gamma = config.get_gamma_map(id);
                        let mut gamma_corrected = crate::app::get_mapped_brightness(slider_value, gamma);

                        // Apply minimum brightness clamp
                        let min_brightness = config.get_min_brightness(id);
                        if gamma_corrected < min_brightness {
                            gamma_corrected = min_brightness;
                        }

                        if let Err(e) = display.set_brightness(gamma_corrected) {
                            tracing::error!("Failed to set brightness on display {}: {}", id, e);
                        } else {
                            synced_count += 1;
                            tracing::debug!("Set brightness to {}% (gamma-corrected from {}%, min {}) on display {} (sync enabled)", gamma_corrected, percentage, min_brightness, id);
                        }
                    } else {
                        tracing::debug!("Skipping brightness sync for display {} (sync disabled)", id);
                    }
                }
                if synced_count > 0 {
                    tracing::debug!("Synced brightness on {} display(s) with gamma correction", synced_count);

                    // Small delay to allow DDC monitors to process the brightness change
                    // This prevents UI flicker from race conditions when reading back brightness
                    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                }
            }
        }

        tracing::warn!("Brightness change stream ended");
        Ok(())
    }
}

/// Spawn the brightness sync daemon if external displays are detected
#[cfg(feature = "brightness-sync-daemon")]
pub async fn spawn_if_needed() {
    match BrightnessSyncDaemon::new().await {
        Ok(Some(daemon)) => {
            // Spawn daemon in background
            tokio::spawn(async move {
                if let Err(e) = daemon.run().await {
                    tracing::error!("Brightness sync daemon error: {}", e);
                }
            });
        }
        Ok(None) => {
            // No external displays, daemon not needed
        }
        Err(e) => {
            tracing::error!("Failed to initialize brightness sync daemon: {}", e);
        }
    }
}

/// No-op when feature is disabled
#[cfg(not(feature = "brightness-sync-daemon"))]
pub async fn spawn_if_needed() {
    // No-op
}
