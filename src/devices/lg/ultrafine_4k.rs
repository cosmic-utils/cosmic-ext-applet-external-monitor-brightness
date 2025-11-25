// SPDX-License-Identifier: GPL-3.0-only
//! LG UltraFine 4K specifications

use crate::devices::{DeviceSpec, Protocol};

/// USB Product ID for LG UltraFine 4K Display (23.7" model)
pub const PRODUCT_ID: u16 = 0x9a63;

/// Device specification for LG UltraFine 4K Display
///
/// Technical specs:
/// - 23.7-inch 4K display (3840 x 2160)
/// - 500 nits brightness
/// - P3 wide color gamut
/// - Thunderbolt 3 / USB-C connectivity
/// - USB HID control via interface 0x7 (Apple HID protocol)
/// - Co-developed with Apple, uses Apple's USB HID protocol
pub const SPEC: DeviceSpec = DeviceSpec {
    product_id: PRODUCT_ID,
    vendor_id: super::VENDOR_ID,
    protocol: Protocol::AppleHid,
    name: "LG UltraFine 4K Display",
    min_brightness_value: 400,
    max_brightness_value: 50000,
    actual_brightness_nits: 500, // 500 nits brightness
};
