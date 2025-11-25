// SPDX-License-Identifier: GPL-3.0-only
//! Apple HID brightness control protocol
//!
//! This protocol is used by Apple displays that communicate via USB HID:
//! - Apple Studio Display
//! - Apple Pro Display XDR
//! - LG UltraFine 4K/5K (co-developed with Apple)
//!
//! Based on the asdbctl implementation:
//! https://github.com/juliuszint/asdbctl

use anyhow::{anyhow, Context, Result};
use hidapi::{HidApi, HidDevice};
use std::sync::{Arc, Mutex};

use super::DisplayProtocol;

/// Apple USB Vendor ID
const APPLE_VENDOR_ID: u16 = 0x05ac;

/// Supported Apple display product IDs
const SUPPORTED_PRODUCT_IDS: &[u16] = &[
    0x1114, // Apple Studio Display
    // Add other Apple/LG displays here as needed
    // 0x9236, // LG UltraFine 5K example
];

/// USB Interface number for brightness control
const INTERFACE_NUMBER: i32 = 0x7;

/// Minimum brightness value in nits
const MIN_BRIGHTNESS_NITS: u32 = 400;

/// Maximum brightness value in nits
const MAX_BRIGHTNESS_NITS: u32 = 60000;

/// Brightness range for calculations
const BRIGHTNESS_RANGE: u32 = MAX_BRIGHTNESS_NITS - MIN_BRIGHTNESS_NITS;

/// HID feature report size in bytes
const REPORT_SIZE: usize = 7;

/// HID Report ID for brightness control
const REPORT_ID: u8 = 1;

/// Apple HID display controller
#[derive(Debug)]
pub struct AppleHidDisplay {
    device: Arc<Mutex<HidDevice>>,
    serial: String,
    manufacturer: String,
    product: String,
}

impl AppleHidDisplay {
    /// Create a new AppleHidDisplay instance from a HID device
    ///
    /// # Arguments
    /// * `device` - The HID device handle
    /// * `serial` - Serial number of the display
    /// * `manufacturer` - Manufacturer string
    /// * `product` - Product name string
    pub fn new(
        device: HidDevice,
        serial: String,
        manufacturer: String,
        product: String,
    ) -> Self {
        Self {
            device: Arc::new(Mutex::new(device)),
            serial,
            manufacturer,
            product,
        }
    }

    /// Enumerate all connected Apple HID displays
    ///
    /// # Arguments
    /// * `api` - HidApi instance for device enumeration
    ///
    /// # Returns
    /// Vector of AppleHidDisplay instances for all connected displays
    pub fn enumerate(api: &HidApi) -> Result<Vec<Self>> {
        let mut displays = Vec::new();

        for device_info in api.device_list() {
            // Check if this is a supported Apple HID display
            if device_info.vendor_id() == APPLE_VENDOR_ID
                && SUPPORTED_PRODUCT_IDS.contains(&device_info.product_id())
                && device_info.interface_number() == INTERFACE_NUMBER
            {
                tracing::debug!(
                    "Found Apple HID display: vendor={:#06x} product={:#06x} interface={} serial={:?}",
                    device_info.vendor_id(),
                    device_info.product_id(),
                    device_info.interface_number(),
                    device_info.serial_number()
                );

                match device_info.open_device(api) {
                    Ok(device) => {
                        let serial = device_info
                            .serial_number()
                            .unwrap_or("Unknown")
                            .to_string();
                        let manufacturer = device_info
                            .manufacturer_string()
                            .unwrap_or("Apple")
                            .to_string();
                        let product = device_info
                            .product_string()
                            .unwrap_or("HID Display")
                            .to_string();

                        tracing::info!("Successfully opened Apple HID display: {}", serial);
                        displays.push(Self::new(device, serial, manufacturer, product));
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to open Apple HID display (serial: {:?}): {}. \
                             This may be a permissions issue. On Linux, ensure udev rules are configured.",
                            device_info.serial_number(),
                            e
                        );
                    }
                }
            }
        }

        if displays.is_empty() {
            tracing::debug!("No Apple HID displays found");
        }

        Ok(displays)
    }
}

impl AppleHidDisplay {
    /// Set brightness without requiring mutable DisplayProtocol trait
    /// This is a convenience method for use outside the trait
    pub fn set_brightness_direct(&self, percentage: u16) -> Result<()> {
        let percentage = percentage.min(100);
        let nits = percentage_to_nits(percentage);

        let device = self
            .device
            .lock()
            .map_err(|e| anyhow!("Failed to lock device: {}", e))?;

        // Prepare buffer for feature report
        let mut buf = [0u8; REPORT_SIZE];
        buf[0] = REPORT_ID;

        // Set brightness value (bytes 1-4, little-endian)
        let nits_bytes = nits.to_le_bytes();
        buf[1..5].copy_from_slice(&nits_bytes);

        // Send feature report
        device
            .send_feature_report(&buf)
            .context("Failed to send HID feature report")?;

        tracing::debug!(
            "Set Apple HID display {} brightness to {}% ({} nits)",
            self.serial,
            percentage,
            nits
        );

        Ok(())
    }
}

impl DisplayProtocol for AppleHidDisplay {
    fn id(&self) -> String {
        format!("apple-hid-{}", self.serial)
    }

    fn name(&self) -> String {
        format!("{} {}", self.manufacturer, self.product)
    }

    fn get_brightness(&mut self) -> Result<u16> {
        let device = self
            .device
            .lock()
            .map_err(|e| anyhow!("Failed to lock device: {}", e))?;

        // Prepare buffer for feature report
        let mut buf = [0u8; REPORT_SIZE];
        buf[0] = REPORT_ID;

        // Read feature report
        device
            .get_feature_report(&mut buf)
            .context("Failed to read HID feature report")?;

        // Extract brightness value (bytes 1-4, little-endian)
        let nits = u32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]);

        // Convert nits to percentage
        let percentage = nits_to_percentage(nits);

        tracing::debug!(
            "Apple HID display {} brightness: {}% ({} nits)",
            self.serial,
            percentage,
            nits
        );

        Ok(percentage)
    }

    fn set_brightness(&mut self, percentage: u16) -> Result<()> {
        let percentage = percentage.min(100);
        let nits = percentage_to_nits(percentage);

        let device = self
            .device
            .lock()
            .map_err(|e| anyhow!("Failed to lock device: {}", e))?;

        // Prepare buffer for feature report
        let mut buf = [0u8; REPORT_SIZE];
        buf[0] = REPORT_ID;

        // Set brightness value (bytes 1-4, little-endian)
        let nits_bytes = nits.to_le_bytes();
        buf[1..5].copy_from_slice(&nits_bytes);

        // Bytes 5-6 are padding (remain 0)

        // Send feature report
        device
            .send_feature_report(&buf)
            .context("Failed to send HID feature report")?;

        tracing::debug!(
            "Set Apple HID display {} brightness to {}% ({} nits)",
            self.serial,
            percentage,
            nits
        );

        Ok(())
    }
}

/// Convert nits value to percentage (0-100)
fn nits_to_percentage(nits: u32) -> u16 {
    if nits <= MIN_BRIGHTNESS_NITS {
        return 0;
    }
    if nits >= MAX_BRIGHTNESS_NITS {
        return 100;
    }

    let percentage = ((nits - MIN_BRIGHTNESS_NITS) as f64 / BRIGHTNESS_RANGE as f64 * 100.0) as u16;
    percentage.min(100)
}

/// Convert percentage (0-100) to nits value
fn percentage_to_nits(percentage: u16) -> u32 {
    let percentage = percentage.min(100);
    let nits = MIN_BRIGHTNESS_NITS + (BRIGHTNESS_RANGE * percentage as u32) / 100;
    nits.clamp(MIN_BRIGHTNESS_NITS, MAX_BRIGHTNESS_NITS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nits_conversion() {
        // Test minimum
        assert_eq!(nits_to_percentage(400), 0);
        assert_eq!(percentage_to_nits(0), 400);

        // Test maximum
        assert_eq!(nits_to_percentage(60000), 100);
        assert_eq!(percentage_to_nits(100), 60000);

        // Test mid-range
        let mid_nits = 30200; // approximately 50%
        let mid_pct = nits_to_percentage(mid_nits);
        assert!(mid_pct >= 49 && mid_pct <= 51);

        // Test round-trip
        for pct in [0, 25, 50, 75, 100] {
            let nits = percentage_to_nits(pct);
            let back_to_pct = nits_to_percentage(nits);
            assert_eq!(pct, back_to_pct);
        }
    }

    #[test]
    fn test_percentage_clamping() {
        assert_eq!(percentage_to_nits(101), MAX_BRIGHTNESS_NITS);
        assert_eq!(percentage_to_nits(200), MAX_BRIGHTNESS_NITS);
    }
}
