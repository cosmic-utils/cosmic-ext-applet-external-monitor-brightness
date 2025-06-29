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
use tokio::sync::watch::Receiver;

use crate::app::Message;

const BRIGHTNESS_CODE: u8 = 0x10;

pub type DisplayId = String;

#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub brightness: f32,
    pub settings_expanded: bool,
}
// impl Monitor {
//     pub fn get_curved_brightness(&self) -> f32 {
//         self.brightness.powf(self.gamma_curve).clamp(0.0, 1.0)
//     }
//     pub fn get_integer_brightness(&self) -> u16 {
//         (self.brightness * 100.0).round() as u16
//     }
//     pub fn get_curved_integer_brightness(&self) -> u16 {
//         (self.get_curved_brightness() * 100.0).round() as u16
//     }
//     pub fn set_integer_brightness(&mut self, brightness: u16) {
//         self.brightness = brightness as f32 / 100.0;
//     }
//     pub fn set_curved_integer_brightness(&mut self, brightness: u16) {
//         self.brightness = (brightness as f32 / 100.0).powf(1.0 / self.gamma_curve);
//     }
//     pub fn change_brightness(&mut self, change: f32) {
//         self.brightness = (self.brightness + change).clamp(0.0, 1.0);
//     }
// }

impl Monitor {
    pub fn get_mapped_brightness(&self, map: f32) -> u16 {
        (self.brightness.powf(map) * 100.0).round() as u16
    }

    pub fn set_mapped_brightness(&mut self, brightness: u16, map: f32) {
        self.brightness = (brightness as f32 / 100.0).powf(1.0 / map);
    }
}

#[derive(Debug, Clone)]
pub enum EventToSub {
    Refresh,
    Set(DisplayId, u16),
}

enum State {
    Waiting,
    Fetch,
    Ready(HashMap<String, Arc<Mutex<Display>>>, Receiver<EventToSub>),
}

pub fn sub() -> impl Stream<Item = Message> {
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
                        let curve = 1.0;
                        let curved_brightness = (brightness as f32 / 100.0).powf(curve);

                        let mon = Monitor {
                            name: display.info.model_name.clone().unwrap_or_default(),
                            brightness: curved_brightness,
                            // gamma_curve: curve,
                            settings_expanded: false,
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

                    let (tx, mut rx) = tokio::sync::watch::channel(EventToSub::Refresh);
                    rx.mark_unchanged();

                    output.send(Message::Ready((res, tx))).await.unwrap();
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
                                    .handle
                                    .get_vcp_feature(BRIGHTNESS_CODE);

                                match res {
                                    Ok(value) => {
                                        output
                                            .send(Message::BrightnessWasUpdated(
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
                        EventToSub::Set(id, value) => {
                            debug_assert!(value <= 100);
                            let display = displays.get_mut(&id).unwrap().clone();

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
    })
}
