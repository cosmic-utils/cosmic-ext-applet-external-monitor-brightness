// SPDX-License-Identifier: GPL-3.0-only
//! Display brightness control protocols
//!
//! This module contains implementations for various display control protocols.
//! Each protocol implementation provides brightness control through different
//! communication methods.

pub mod ddc_ci;

#[cfg(feature = "apple-hid-displays")]
pub mod apple_hid;

use anyhow::Result;

/// Common trait for all display control protocols
pub trait DisplayProtocol: std::fmt::Debug + Send {
    /// Get the unique identifier for this display
    fn id(&self) -> String;

    /// Get the human-readable name of this display
    fn name(&self) -> String;

    /// Get the current brightness (0-100)
    fn get_brightness(&mut self) -> Result<u16>;

    /// Set the brightness (0-100)
    fn set_brightness(&mut self, value: u16) -> Result<()>;
}
