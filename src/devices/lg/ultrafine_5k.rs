// SPDX-License-Identifier: GPL-3.0-only
//! LG UltraFine 5K specifications

use crate::devices::{DeviceSpec, Protocol};

/// USB Product ID for LG UltraFine 5K Display
pub const PRODUCT_ID: u16 = 0x9a70;

/// Device specification for LG UltraFine 5K Display
///
/// Technical specs:
/// - 27-inch 5K display (5120 x 2880)
/// - 500 nits brightness
/// - P3 wide color gamut (99%), 10-bit depth
/// - Thunderbolt 3 connectivity
/// - USB HID control via interface 0x7 (Apple HID protocol)
/// - Co-developed with Apple, uses Apple's USB HID protocol
pub const SPEC: DeviceSpec = DeviceSpec {
    product_id: PRODUCT_ID,
    vendor_id: super::VENDOR_ID,
    protocol: Protocol::AppleHid,
    name: "LG UltraFine 5K Display",
    min_brightness_value: 400,
    max_brightness_value: 50000,
    actual_brightness_nits: 500, // 500 nits brightness
};
