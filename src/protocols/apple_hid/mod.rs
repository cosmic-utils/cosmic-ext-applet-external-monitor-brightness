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

mod device;

pub use device::AppleHidDisplay;

/// Apple USB Vendor ID
pub const VENDOR_ID: u16 = 0x05ac;

/// USB Interface number for brightness control
pub const INTERFACE_NUMBER: i32 = 0x7;
