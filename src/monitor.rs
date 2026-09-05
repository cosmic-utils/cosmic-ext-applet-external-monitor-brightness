use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use cosmic::iced::{
    futures::{SinkExt, Stream},
    stream,
};
use ddc_hi::{Ddc, Display};

use crate::app::AppMsg;

const BRIGHTNESS_CODE: u8 = 0x10;

pub type DisplayId = String;
pub type ScreenBrightness = u16;

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub brightness: u16,
}

#[derive(Debug, Clone)]
pub enum EventToSub {
    /// Refresh brightness values using the cached display handles.
    RefreshBrightness,
    /// Re-enumerate connected displays and replace the cached display handles.
    RescanDisplays,
    Set(DisplayId, ScreenBrightness),
}

enum State {
    Waiting,
    Fetch,
    Ready(HashMap<DisplayId, Arc<Mutex<Display>>>),
}

pub fn sub() -> impl Stream<Item = AppMsg> {
    stream::channel(
        100,
        |mut output: cosmic::iced::futures::channel::mpsc::Sender<AppMsg>| async move {
            // Keep the receiver outside `State` so rescans can leave `Ready` without
            // disconnecting the sender while enumeration is retried.
            let (tx, mut rx) = tokio::sync::watch::channel(EventToSub::RefreshBrightness);
            rx.mark_unchanged();

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
                        for mut display in Display::enumerate() {
                            let brightness = match display.handle.get_vcp_feature(BRIGHTNESS_CODE) {
                                Ok(v) => v.value(),
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

                            let mon = MonitorInfo {
                                name: display.info.model_name.clone().unwrap_or_default(),
                                brightness,
                            };

                            res.insert(display.info.id.clone(), mon);
                            displays.insert(display.info.id.clone(), Arc::new(Mutex::new(display)));
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

                        output
                            .send(AppMsg::SubscriptionReady((res, tx.clone())))
                            .await
                            .unwrap();
                        state = State::Ready(displays);
                    }
                    State::Ready(displays) => {
                        rx.changed().await.unwrap();

                        let last = rx.borrow_and_update().clone();
                        match last {
                            EventToSub::RefreshBrightness => {
                                for (id, display) in displays {
                                    let res = display
                                        .lock()
                                        .unwrap()
                                        .handle
                                        .get_vcp_feature(BRIGHTNESS_CODE);

                                    match res {
                                        Ok(value) => {
                                            output
                                                .send(AppMsg::BrightnessWasUpdated(
                                                    id.clone(),
                                                    value.value(),
                                                ))
                                                .await
                                                .unwrap();
                                        }
                                        Err(err) => error!("{:?}", err),
                                    }
                                }
                            }
                            EventToSub::RescanDisplays => {
                                // The Fetch state performs a full DDC enumeration and sends the
                                // resulting monitor list back to the application.
                                // Start each rescan with a fresh retry budget.
                                failed_attempts = 0;
                                duration = Duration::from_millis(50);
                                state = State::Fetch;
                            }
                            EventToSub::Set(id, value) => {
                                debug_assert!(value <= 100);
                                let display = Arc::clone(displays.get_mut(&id).unwrap());

                                let j = tokio::task::spawn_blocking(move || {
                                    if let Err(err) = display
                                        .lock()
                                        .unwrap()
                                        .handle
                                        .set_vcp_feature(BRIGHTNESS_CODE, value)
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
        },
    )
}
