use ddc_hi::{Ddc, Display};

const BRIGHTNESS_CODE: u8 = 0x10;

pub struct Monitor {
    pub display: Display,
    pub brightness: u16,
}

pub fn init() -> Vec<Monitor> {
    let mut monitors: Vec<Monitor> = vec![];
    for display in Display::enumerate() {
        let mut mon = Monitor::new(display);
        mon.update_brightness();

        monitors.push(mon);
    }
    monitors
}

impl Monitor {
    pub fn new(display: Display) -> Self {
        Monitor {
            display,
            brightness: 0,
        }
    }

    pub fn set_screen_brightness(&mut self, brightness: u16) {
        self.brightness = brightness;
        let _ = self
            .display
            .handle
            .set_vcp_feature(BRIGHTNESS_CODE, brightness);
    }

    pub fn update_brightness(&mut self) {
        let start = std::time::Instant::now();

        self.brightness = self
            .display
            .handle
            .get_vcp_feature(BRIGHTNESS_CODE)
            .map(|v| v.value())
            .unwrap_or_default();

        let duration = start.elapsed();

        log::debug!(
            "update_brightness for display {}: {:?}",
            self.display.info,
            duration
        );
    }
}
