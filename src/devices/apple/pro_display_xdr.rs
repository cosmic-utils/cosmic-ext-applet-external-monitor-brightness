// SPDX-License-Identifier: GPL-3.0-only
//! Apple Pro Display XDR specifications

use crate::devices::{DeviceSpec, Protocol};

/// USB Product ID for Apple Pro Display XDR
pub const PRODUCT_ID: u16 = 0x9243;

/// Device specification for Apple Pro Display XDR
///
/// Technical specs:
/// - 32-inch Retina 6K display (6016 x 3384)
/// - 1000 nits sustained full-screen brightness
/// - 1600 nits peak brightness
/// - P3 wide color gamut, 10-bit depth
/// - USB HID control via interface 0x7
pub const SPEC: DeviceSpec = DeviceSpec {
    product_id: PRODUCT_ID,
    vendor_id: super::VENDOR_ID,
    protocol: Protocol::AppleHid,
    name: "Apple Pro Display XDR",
    min_brightness_value: 400,
    max_brightness_value: 50000,
    actual_brightness_nits: 1600, // 1600 nits peak brightness
};
