// SPDX-License-Identifier: GPL-3.0-only
//! Device-specific configurations organized by manufacturer

pub mod apple;
pub mod lg;

/// Display communication protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// Apple HID protocol (used by Apple and LG displays)
    AppleHid,
    /// DDC/CI protocol (standard for most external monitors)
    DdcCi,
}

/// Device specification for a display
#[derive(Debug, Clone)]
pub struct DeviceSpec {
    /// USB Product ID
    #[allow(dead_code)]
    pub product_id: u16,

    /// USB Vendor ID
    pub vendor_id: u16,

    /// Communication protocol
    pub protocol: Protocol,

    /// Human-readable device name
    pub name: &'static str,

    /// Minimum brightness protocol value (not physical nits)
    /// This is the raw value sent to the device via its control protocol
    pub min_brightness_value: u32,

    /// Maximum brightness protocol value (not physical nits)
    /// This is the raw value sent to the device via its control protocol
    pub max_brightness_value: u32,

    /// Actual maximum brightness capability in nits (physical measurement)
    /// This is for documentation and user information only
    pub actual_brightness_nits: u16,
}

impl DeviceSpec {
    /// Get the brightness protocol value range (max - min)
    pub fn brightness_range(&self) -> u32 {
        self.max_brightness_value - self.min_brightness_value
    }
}

/// Get device specification by product ID
pub fn get_device_spec(product_id: u16) -> Option<DeviceSpec> {
    match product_id {
        apple::studio_display::PRODUCT_ID => Some(apple::studio_display::SPEC),
        apple::pro_display_xdr::PRODUCT_ID => Some(apple::pro_display_xdr::SPEC),
        lg::ultrafine_4k::PRODUCT_ID => Some(lg::ultrafine_4k::SPEC),
        lg::ultrafine_5k::PRODUCT_ID => Some(lg::ultrafine_5k::SPEC),
        _ => None,
    }
}

/// Get all supported product IDs
pub fn supported_product_ids() -> Vec<u16> {
    vec![
        apple::studio_display::PRODUCT_ID,
        apple::pro_display_xdr::PRODUCT_ID,
        lg::ultrafine_4k::PRODUCT_ID,
        lg::ultrafine_5k::PRODUCT_ID,
    ]
}
