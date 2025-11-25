// SPDX-License-Identifier: GPL-3.0-only
//! Apple HID display device implementation

use anyhow::{anyhow, Context, Result};
use hidapi::{HidApi, HidDevice};
use std::sync::{Arc, Mutex};

use crate::protocols::DisplayProtocol;
use crate::devices::{self, get_device_spec, supported_product_ids, DeviceSpec};

use super::{VENDOR_ID, INTERFACE_NUMBER};

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
    /// Device-specific specification (brightness ranges, etc.)
    spec: DeviceSpec,
}

impl AppleHidDisplay {
    /// Create a new AppleHidDisplay instance from a HID device
    ///
    /// # Arguments
    /// * `device` - The HID device handle
    /// * `serial` - Serial number of the display
    /// * `manufacturer` - Manufacturer string
    /// * `product` - Product name string
    /// * `spec` - Device specification with brightness ranges
    pub fn new(
        device: HidDevice,
        serial: String,
        manufacturer: String,
        product: String,
        spec: DeviceSpec,
    ) -> Self {
        Self {
            device: Arc::new(Mutex::new(device)),
            serial,
            manufacturer,
            product,
            spec,
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
        let supported_ids = supported_product_ids();

        for device_info in api.device_list() {
            let vendor_id = device_info.vendor_id();
            let product_id = device_info.product_id();

            // Check if this is a supported Apple HID display (Apple or LG)
            let is_apple = vendor_id == VENDOR_ID;  // Apple vendor ID
            let is_lg = vendor_id == devices::lg::VENDOR_ID;  // LG vendor ID

            if (is_apple || is_lg)
                && supported_ids.contains(&product_id)
                && device_info.interface_number() == INTERFACE_NUMBER
            {
                // Get device specification
                let spec = match get_device_spec(product_id) {
                    Some(spec) => spec,
                    None => {
                        tracing::warn!("No device spec found for product ID {:#06x}", product_id);
                        continue;
                    }
                };

                tracing::debug!(
                    "Found HID display: vendor={:#06x} product={:#06x} ({}) interface={} serial={:?}",
                    vendor_id,
                    product_id,
                    spec.name,
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

                        tracing::info!("Successfully opened {} (serial: {})", spec.name, serial);
                        displays.push(Self::new(device, serial, manufacturer, product, spec));
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to open {} (serial: {:?}): {}. \
                             This may be a permissions issue. On Linux, ensure udev rules are configured.",
                            spec.name,
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
    /// Convert percentage (0-100) to protocol value for this device
    fn percentage_to_protocol_value(&self, percentage: u16) -> u32 {
        let percentage = percentage.min(100);
        let min_value = self.spec.min_brightness_value;
        let range = self.spec.brightness_range();
        min_value + (range * percentage as u32) / 100
    }

    /// Convert protocol value to percentage (0-100) for this device
    fn protocol_value_to_percentage(&self, value: u32) -> u16 {
        let min_value = self.spec.min_brightness_value;
        let max_value = self.spec.max_brightness_value;

        if value <= min_value {
            return 0;
        }
        if value >= max_value {
            return 100;
        }

        let range = self.spec.brightness_range();
        let percentage = ((value - min_value) as f64 / range as f64 * 100.0) as u16;
        percentage.min(100)
    }

    /// Set brightness without requiring mutable DisplayProtocol trait
    /// This is a convenience method for use outside the trait
    pub fn set_brightness_direct(&self, percentage: u16) -> Result<()> {
        let percentage = percentage.min(100);
        let value = self.percentage_to_protocol_value(percentage);

        let device = self
            .device
            .lock()
            .map_err(|e| anyhow!("Failed to lock device: {}", e))?;

        // Prepare buffer for feature report
        let mut buf = [0u8; REPORT_SIZE];
        buf[0] = REPORT_ID;

        // Set brightness value (bytes 1-4, little-endian)
        let value_bytes = value.to_le_bytes();
        buf[1..5].copy_from_slice(&value_bytes);

        // Send feature report
        device
            .send_feature_report(&buf)
            .context("Failed to send HID feature report")?;

        tracing::debug!(
            "Set {} {} brightness to {}% (protocol value: {})",
            self.spec.name,
            self.serial,
            percentage,
            value
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
        let value = u32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]);

        // Convert protocol value to percentage using device-specific range
        let percentage = self.protocol_value_to_percentage(value);

        tracing::debug!(
            "{} {} brightness: {}% (protocol value: {})",
            self.spec.name,
            self.serial,
            percentage,
            value
        );

        Ok(percentage)
    }

    fn set_brightness(&mut self, percentage: u16) -> Result<()> {
        let percentage = percentage.min(100);
        let value = self.percentage_to_protocol_value(percentage);

        let device = self
            .device
            .lock()
            .map_err(|e| anyhow!("Failed to lock device: {}", e))?;

        // Prepare buffer for feature report
        let mut buf = [0u8; REPORT_SIZE];
        buf[0] = REPORT_ID;

        // Set brightness value (bytes 1-4, little-endian)
        let value_bytes = value.to_le_bytes();
        buf[1..5].copy_from_slice(&value_bytes);

        // Bytes 5-6 are padding (remain 0)

        // Send feature report
        device
            .send_feature_report(&buf)
            .context("Failed to send HID feature report")?;

        tracing::debug!(
            "Set {} {} brightness to {}% (protocol value: {})",
            self.spec.name,
            self.serial,
            percentage,
            value
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::apple::{studio_display, pro_display_xdr};

    #[test]
    fn test_studio_display_protocol_values() {
        // Create a mock Studio Display spec
        let spec = studio_display::SPEC;

        // Test protocol value range
        assert_eq!(spec.min_brightness_value, 400);
        assert_eq!(spec.max_brightness_value, 60000);

        // Test range calculation
        assert_eq!(spec.brightness_range(), 59600);
    }

    #[test]
    fn test_pro_display_xdr_protocol_values() {
        // Create a mock Pro Display XDR spec
        let spec = pro_display_xdr::SPEC;

        // Test protocol value range
        assert_eq!(spec.min_brightness_value, 400);
        assert_eq!(spec.max_brightness_value, 50000);

        // Test range calculation
        assert_eq!(spec.brightness_range(), 49600);
    }

    #[test]
    fn test_device_spec_lookup() {
        use crate::devices::Protocol;

        // Test Apple Studio Display lookup
        let studio_spec = get_device_spec(0x1114).expect("Studio Display spec not found");
        assert_eq!(studio_spec.product_id, 0x1114);
        assert_eq!(studio_spec.vendor_id, 0x05ac); // Apple vendor ID
        assert_eq!(studio_spec.protocol, Protocol::AppleHid);
        assert_eq!(studio_spec.name, "Apple Studio Display");
        assert_eq!(studio_spec.max_brightness_value, 60000);

        // Test Apple Pro Display XDR lookup
        let xdr_spec = get_device_spec(0x9243).expect("Pro Display XDR spec not found");
        assert_eq!(xdr_spec.product_id, 0x9243);
        assert_eq!(xdr_spec.vendor_id, 0x05ac); // Apple vendor ID
        assert_eq!(xdr_spec.protocol, Protocol::AppleHid);
        assert_eq!(xdr_spec.name, "Apple Pro Display XDR");
        assert_eq!(xdr_spec.max_brightness_value, 50000);

        // Test LG UltraFine 4K lookup
        let lg_4k_spec = get_device_spec(0x9a63).expect("LG UltraFine 4K spec not found");
        assert_eq!(lg_4k_spec.product_id, 0x9a63);
        assert_eq!(lg_4k_spec.vendor_id, 0x043e); // LG vendor ID
        assert_eq!(lg_4k_spec.protocol, Protocol::AppleHid);
        assert_eq!(lg_4k_spec.name, "LG UltraFine 4K Display");
        assert_eq!(lg_4k_spec.max_brightness_value, 50000);

        // Test LG UltraFine 5K lookup
        let lg_5k_spec = get_device_spec(0x9a70).expect("LG UltraFine 5K spec not found");
        assert_eq!(lg_5k_spec.product_id, 0x9a70);
        assert_eq!(lg_5k_spec.vendor_id, 0x043e); // LG vendor ID
        assert_eq!(lg_5k_spec.protocol, Protocol::AppleHid);
        assert_eq!(lg_5k_spec.name, "LG UltraFine 5K Display");
        assert_eq!(lg_5k_spec.max_brightness_value, 50000);

        // Test unknown product ID
        assert!(get_device_spec(0xFFFF).is_none());
    }

    #[test]
    fn test_supported_product_ids() {
        let ids = supported_product_ids();
        assert!(ids.contains(&0x1114)); // Apple Studio Display
        assert!(ids.contains(&0x9243)); // Apple Pro Display XDR
        assert!(ids.contains(&0x9a63)); // LG UltraFine 4K
        assert!(ids.contains(&0x9a70)); // LG UltraFine 5K
        assert_eq!(ids.len(), 4);
    }
}
