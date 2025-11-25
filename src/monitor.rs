use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use cosmic::iced::{
    futures::{SinkExt, Stream},
    stream,
};
use tokio::sync::watch::Receiver;

use crate::app::AppMsg;
use crate::protocols::{ddc_ci::DdcCiDisplay, DisplayProtocol};

#[cfg(feature = "apple-studio-display")]
use crate::protocols::apple_hid::AppleHidDisplay;

pub type DisplayId = String;
pub type ScreenBrightness = u16;

/// Backend type for display control
pub enum DisplayBackend {
    /// DDC/CI protocol (standard external monitors via I2C)
    DdcCi(DdcCiDisplay),
    /// Apple HID protocol (Apple Studio Display, LG UltraFine, etc.)
    #[cfg(feature = "apple-studio-display")]
    AppleHid(AppleHidDisplay),
}

impl std::fmt::Debug for DisplayBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayBackend::DdcCi(display) => write!(f, "{:?}", display),
            #[cfg(feature = "apple-studio-display")]
            DisplayBackend::AppleHid(display) => write!(f, "{:?}", display),
        }
    }
}

impl DisplayBackend {
    /// Get the display ID
    pub fn id(&self) -> String {
        match self {
            DisplayBackend::DdcCi(display) => display.id(),
            #[cfg(feature = "apple-studio-display")]
            DisplayBackend::AppleHid(display) => display.id(),
        }
    }

    /// Get the display name
    pub fn name(&self) -> String {
        match self {
            DisplayBackend::DdcCi(display) => display.name(),
            #[cfg(feature = "apple-studio-display")]
            DisplayBackend::AppleHid(display) => display.name(),
        }
    }

    /// Get the current brightness (0-100)
    pub fn get_brightness(&mut self) -> anyhow::Result<u16> {
        match self {
            DisplayBackend::DdcCi(display) => display.get_brightness(),
            #[cfg(feature = "apple-studio-display")]
            DisplayBackend::AppleHid(display) => display.get_brightness(),
        }
    }

    /// Set the brightness (0-100)
    pub fn set_brightness(&mut self, value: u16) -> anyhow::Result<()> {
        match self {
            DisplayBackend::DdcCi(display) => display.set_brightness(value),
            #[cfg(feature = "apple-studio-display")]
            DisplayBackend::AppleHid(display) => display.set_brightness(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub brightness: u16,
}

#[derive(Debug, Clone)]
pub enum EventToSub {
    Refresh,
    Set(DisplayId, ScreenBrightness),
}

enum State {
    Waiting,
    Fetch,
    Ready(
        HashMap<DisplayId, Arc<Mutex<DisplayBackend>>>,
        Receiver<EventToSub>,
    ),
}

pub fn sub() -> impl Stream<Item = AppMsg> {
    stream::channel(100, |mut output| async move {
        let mut state = State::Waiting;
        let mut failed_attempts = 0;

        let mut duration = Duration::from_millis(50);

        loop {
            match &mut state {
                State::Waiting => {
                    tokio::time::sleep(duration).await;
                    duration *= 2;
                    state = State::Fetch;
                }
                State::Fetch => {
                    let mut res = HashMap::new();

                    let mut displays = HashMap::new();

                    debug!("start enumerate");

                    let mut some_failed = false;

                    // Enumerate DDC/CI displays
                    for display in DdcCiDisplay::enumerate() {
                        let mut backend = DisplayBackend::DdcCi(display);

                        let brightness = match backend.get_brightness() {
                            Ok(v) => v,
                            // on my machine, i get this error when starting the session
                            // can't get_vcp_feature: DDC/CI error: Expected DDC/CI length bit
                            // This go away after the third attempt
                            Err(e) => {
                                error!("can't get_vcp_feature: {e}");
                                some_failed = true;
                                continue;
                            }
                        };
                        debug_assert!(brightness <= 100);

                        let id = backend.id();
                        let name = backend.name();

                        let mon = MonitorInfo {
                            name,
                            brightness,
                        };

                        res.insert(id.clone(), mon);
                        displays.insert(id, Arc::new(Mutex::new(backend)));
                    }

                    // Enumerate Apple HID displays
                    #[cfg(feature = "apple-studio-display")]
                    {
                        match hidapi::HidApi::new() {
                            Ok(api) => {
                                match AppleHidDisplay::enumerate(&api) {
                                    Ok(apple_displays) => {
                                        for display in apple_displays {
                                            let mut backend = DisplayBackend::AppleHid(display);

                                            let brightness = match backend.get_brightness() {
                                                Ok(v) => v,
                                                Err(e) => {
                                                    error!("can't get Apple HID display brightness: {e}");
                                                    some_failed = true;
                                                    continue;
                                                }
                                            };

                                            let id = backend.id();
                                            let name = backend.name();

                                            let mon = MonitorInfo {
                                                name,
                                                brightness,
                                            };

                                            res.insert(id.clone(), mon);
                                            displays.insert(id, Arc::new(Mutex::new(backend)));
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to enumerate Apple HID displays: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to initialize HID API: {e}");
                            }
                        }
                    }

                    if some_failed {
                        failed_attempts += 1;
                    }

                    // On some monitors this error is permanent
                    // So we mark the app as ready if at least one monitor is loaded after 5 attempts
                    if some_failed && failed_attempts < 5 {
                        state = State::Waiting;
                        continue;
                    }

                    debug!("end enumerate");

                    let (tx, mut rx) = tokio::sync::watch::channel(EventToSub::Refresh);
                    rx.mark_unchanged();

                    output
                        .send(AppMsg::SubscriptionReady((res, tx)))
                        .await
                        .unwrap();
                    state = State::Ready(displays, rx);
                }
                State::Ready(displays, rx) => {
                    rx.changed().await.unwrap();

                    let last = rx.borrow_and_update().clone();
                    match last {
                        EventToSub::Refresh => {
                            for (id, display) in displays {
                                let res = display
                                    .lock()
                                    .unwrap()
                                    .get_brightness();

                                match res {
                                    Ok(value) => {
                                        output
                                            .send(AppMsg::BrightnessWasUpdated(
                                                id.clone(),
                                                value,
                                            ))
                                            .await
                                            .unwrap();
                                    }
                                    Err(err) => error!("{:?}", err),
                                }
                            }
                        }
                        EventToSub::Set(id, value) => {
                            debug_assert!(value <= 100);
                            let display = displays.get_mut(&id).unwrap().clone();

                            let j = tokio::task::spawn_blocking(move || {
                                if let Err(err) = display
                                    .lock()
                                    .unwrap()
                                    .set_brightness(value)
                                {
                                    error!("{:?}", err);
                                }
                            });

                            j.await.unwrap();
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                    }
                }
            }
        }
    })
}
