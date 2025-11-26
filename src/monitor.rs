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

#[cfg(feature = "apple-hid-displays")]
use crate::protocols::apple_hid::AppleHidDisplay;

pub type DisplayId = String;
pub type ScreenBrightness = u16;

/// Backend type for display control
pub enum DisplayBackend {
    /// DDC/CI protocol (standard external monitors via I2C)
    DdcCi(DdcCiDisplay),
    /// Apple HID protocol (Apple Studio Display, LG UltraFine, etc.)
    #[cfg(feature = "apple-hid-displays")]
    AppleHid(AppleHidDisplay),
}

impl std::fmt::Debug for DisplayBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayBackend::DdcCi(display) => write!(f, "{:?}", display),
            #[cfg(feature = "apple-hid-displays")]
            DisplayBackend::AppleHid(display) => write!(f, "{:?}", display),
        }
    }
}

impl DisplayBackend {
    /// Get the display ID
    pub fn id(&self) -> String {
        match self {
            DisplayBackend::DdcCi(display) => display.id(),
            #[cfg(feature = "apple-hid-displays")]
            DisplayBackend::AppleHid(display) => display.id(),
        }
    }

    /// Get the display name
    pub fn name(&self) -> String {
        match self {
            DisplayBackend::DdcCi(display) => display.name(),
            #[cfg(feature = "apple-hid-displays")]
            DisplayBackend::AppleHid(display) => display.name(),
        }
    }

    /// Get the current brightness (0-100)
    pub fn get_brightness(&mut self) -> anyhow::Result<u16> {
        match self {
            DisplayBackend::DdcCi(display) => display.get_brightness(),
            #[cfg(feature = "apple-hid-displays")]
            DisplayBackend::AppleHid(display) => display.get_brightness(),
        }
    }

    /// Set the brightness (0-100)
    pub fn set_brightness(&mut self, value: u16) -> anyhow::Result<()> {
        match self {
            DisplayBackend::DdcCi(display) => display.set_brightness(value),
            #[cfg(feature = "apple-hid-displays")]
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
    ReEnumerate,
}

enum State {
    Waiting,
    Fetch(Option<tokio::sync::watch::Sender<EventToSub>>),
    Ready(
        HashMap<DisplayId, Arc<Mutex<DisplayBackend>>>,
        tokio::sync::watch::Sender<EventToSub>,
        Receiver<EventToSub>,
    ),
}

pub fn sub() -> impl Stream<Item = AppMsg> {
    stream::channel(100, |mut output| async move {
        let mut state = State::Fetch(None); // Start immediately, no waiting
        let mut failed_attempts = 0;

        loop {
            match &mut state {
                State::Waiting => {
                    // Only wait 100ms between retries, no exponential backoff
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    state = State::Fetch(None);
                }
                State::Fetch(existing_sender) => {
                    let mut res = HashMap::new();

                    let mut displays = HashMap::new();

                    debug!("start enumerate");

                    let mut some_failed = false;

                    // Enumerate DDC/CI displays concurrently
                    let ddc_displays = DdcCiDisplay::enumerate();
                    let mut ddc_tasks = Vec::new();

                    for display in ddc_displays {
                        let task = tokio::spawn(async move {
                            let mut backend = DisplayBackend::DdcCi(display);

                            // Wake up DDC by doing a read-write cycle
                            // Some DDC monitors need an initial write to establish I2C communication
                            if let Ok(current_brightness) = backend.get_brightness() {
                                let _ = backend.set_brightness(current_brightness);
                                // Small delay to let DDC settle
                                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            }

                            let brightness = match backend.get_brightness() {
                                Ok(v) => v,
                                // on my machine, i get this error when starting the session
                                // can't get_vcp_feature: DDC/CI error: Expected DDC/CI length bit
                                // This go away after the third attempt
                                Err(e) => {
                                    error!("can't get_vcp_feature: {e}");
                                    return Err(e);
                                }
                            };
                            debug_assert!(brightness <= 100);

                            let id = backend.id();
                            let name = backend.name();

                            let mon = MonitorInfo {
                                name,
                                brightness,
                            };

                            Ok((id, mon, backend))
                        });
                        ddc_tasks.push(task);
                    }

                    // Wait for all DDC tasks to complete
                    for task in ddc_tasks {
                        match task.await {
                            Ok(Ok((id, mon, backend))) => {
                                res.insert(id.clone(), mon);
                                displays.insert(id, Arc::new(Mutex::new(backend)));
                            }
                            Ok(Err(_)) => {
                                some_failed = true;
                            }
                            Err(e) => {
                                error!("Task join error: {e}");
                                some_failed = true;
                            }
                        }
                    }

                    // Enumerate Apple HID displays
                    #[cfg(feature = "apple-hid-displays")]
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

                    // If we have at least one monitor, send it to the UI immediately
                    // and retry failed monitors in the background
                    if !res.is_empty() {
                        // We have at least one working monitor, proceed to ready state
                    } else if some_failed && failed_attempts < 3 {
                        // No monitors detected yet, retry up to 3 times
                        state = State::Waiting;
                        continue;
                    }

                    debug!("end enumerate");

                    let (tx, mut rx) = if let Some(sender) = existing_sender.take() {
                        // Reuse existing sender for re-enumeration
                        let rx = sender.subscribe();
                        (sender, rx)
                    } else {
                        // Create new channel for initial enumeration
                        let (tx, mut rx) = tokio::sync::watch::channel(EventToSub::Refresh);
                        rx.mark_unchanged();
                        (tx, rx)
                    };

                    output
                        .send(AppMsg::SubscriptionReady((res, tx.clone())))
                        .await
                        .unwrap();
                    state = State::Ready(displays, tx, rx);
                }
                State::Ready(displays, tx, rx) => {
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
                        EventToSub::ReEnumerate => {
                            // Transition back to Fetch state with existing sender
                            // This will re-enumerate displays while keeping the same channel
                            state = State::Fetch(Some(tx.clone()));
                        }
                    }
                }
            }
        }
    })
}
