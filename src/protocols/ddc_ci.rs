// SPDX-License-Identifier: GPL-3.0-only
//! DDC/CI (Display Data Channel Command Interface) protocol implementation
//!
//! DDC/CI is a standard protocol for controlling monitors over I2C bus.
//! It's supported by most modern external monitors via the video cable.

use anyhow::Result;
use ddc_hi::{Ddc, Display};

use super::DisplayProtocol;

/// VCP (Virtual Control Panel) code for brightness
const BRIGHTNESS_CODE: u8 = 0x10;

/// DDC/CI display implementation
pub struct DdcCiDisplay {
    display: Display,
}

impl DdcCiDisplay {
    /// Create a new DDC/CI display wrapper
    pub fn new(display: Display) -> Self {
        Self { display }
    }

    /// Enumerate all DDC/CI displays
    pub fn enumerate() -> Vec<Self> {
        Display::enumerate()
            .into_iter()
            .map(Self::new)
            .collect()
    }
}

impl DisplayProtocol for DdcCiDisplay {
    fn id(&self) -> String {
        self.display.info.id.clone()
    }

    fn name(&self) -> String {
        self.display
            .info
            .model_name
            .clone()
            .unwrap_or_default()
    }

    fn get_brightness(&mut self) -> Result<u16> {
        let value = self.display.handle.get_vcp_feature(BRIGHTNESS_CODE)?;
        Ok(value.value())
    }

    fn set_brightness(&mut self, value: u16) -> Result<()> {
        self.display
            .handle
            .set_vcp_feature(BRIGHTNESS_CODE, value)?;
        Ok(())
    }
}

impl std::fmt::Debug for DdcCiDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DdcCiDisplay(id: {}, name: {})", self.id(), self.name())
    }
}
