// SPDX-License-Identifier: GPL-3.0-only
//! Brightness synchronization daemon
//!
//! This daemon listens to COSMIC's DisplayBrightness changes and
//! applies them to displays using the Apple HID protocol via USB.
//!
//! Supports:
//! - Apple Studio Display
//! - Apple Pro Display XDR
//! - LG UltraFine 4K
//! - LG UltraFine 5K
//!
//! Only activates when Apple HID protocol displays are detected.

#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
use anyhow::{Context, Result};
#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
use std::sync::Arc;
#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
use tokio::sync::Mutex;
#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
use zbus::{proxy, Connection};

#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
use crate::protocols::apple_hid::AppleHidDisplay;

#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
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

#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
pub struct BrightnessSyncDaemon {
    apple_displays: Arc<Mutex<Vec<AppleHidDisplay>>>,
}

#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
impl BrightnessSyncDaemon {
    /// Create a new brightness sync daemon
    /// Returns None if no Apple HID protocol displays are detected
    pub async fn new() -> Result<Option<Self>> {
        // Enumerate Apple HID protocol displays (Apple and LG)
        let api = hidapi::HidApi::new().context("Failed to initialize HID API")?;
        let displays = AppleHidDisplay::enumerate(&api)
            .context("Failed to enumerate Apple HID displays")?;

        if displays.is_empty() {
            tracing::info!("No Apple HID displays detected, brightness sync disabled");
            return Ok(None);
        }

        tracing::info!(
            "Found {} Apple HID display(s), enabling brightness sync daemon",
            displays.len()
        );

        Ok(Some(Self {
            apple_displays: Arc::new(Mutex::new(displays)),
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

        // Get current brightness and apply it to Apple displays on startup
        let current_brightness = proxy
            .display_brightness()
            .await
            .context("Failed to get current display brightness")?;

        let percentage = if max_brightness > 0 {
            ((current_brightness as f64 / max_brightness as f64) * 100.0) as u16
        } else {
            0
        };
        let percentage = percentage.min(100);

        tracing::info!(
            "Initial sync: applying {}% to Apple displays (COSMIC value: {}/{})",
            percentage,
            current_brightness,
            max_brightness
        );

        // Apply current brightness to all Apple displays
        {
            let displays = self.apple_displays.lock().await;
            for display in displays.iter() {
                if let Err(e) = display.set_brightness_direct(percentage) {
                    tracing::error!("Failed to set initial brightness on Apple display: {}", e);
                }
            }
        }

        // Subscribe to DisplayBrightness property changes
        use futures::StreamExt;
        let mut brightness_changed = proxy.receive_display_brightness_changed().await;

        tracing::info!("Listening for COSMIC brightness changes...");

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
                    "Applying {}% to Apple displays (COSMIC value: {}/{})",
                    percentage,
                    brightness,
                    max_brightness
                );

                // Apply to all Apple displays
                let displays = self.apple_displays.lock().await;
                for display in displays.iter() {
                    if let Err(e) = display.set_brightness_direct(percentage) {
                        tracing::error!("Failed to set brightness on Apple display: {}", e);
                    }
                }
            }
        }

        tracing::warn!("Brightness change stream ended");
        Ok(())
    }
}

/// Spawn the brightness sync daemon if Apple displays are detected
#[cfg(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon"))]
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
            // No Apple displays, daemon not needed
        }
        Err(e) => {
            tracing::error!("Failed to initialize brightness sync daemon: {}", e);
        }
    }
}

/// No-op when feature is disabled
#[cfg(not(all(feature = "apple-hid-displays", feature = "brightness-sync-daemon")))]
pub async fn spawn_if_needed() {
    // No-op
}
