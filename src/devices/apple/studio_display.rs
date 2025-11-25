// SPDX-License-Identifier: GPL-3.0-only
//! Apple Studio Display specifications

use crate::devices::{DeviceSpec, Protocol};

/// USB Product ID for Apple Studio Display
pub const PRODUCT_ID: u16 = 0x1114;

/// Device specification for Apple Studio Display
pub const SPEC: DeviceSpec = DeviceSpec {
    product_id: PRODUCT_ID,
    vendor_id: super::VENDOR_ID,
    protocol: Protocol::AppleHid,
    name: "Apple Studio Display",
    min_brightness_value: 400,
    max_brightness_value: 60000,
    actual_brightness_nits: 600, // 600 nits SDR brightness
};
