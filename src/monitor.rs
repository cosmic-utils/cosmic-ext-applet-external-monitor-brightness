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

use crate::window::Message;

const BRIGHTNESS_CODE: u8 = 0x10;

pub type DisplayId = String;

#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub brightness: u16,
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

                    for mut display in Display::enumerate() {
                        let brightness = match display.handle.get_vcp_feature(BRIGHTNESS_CODE) {
                            Ok(v) => v.value(),
                            Err(e) => {
                                // on my machine, i get this error when starting the session
                                // can't get_vcp_feature: DDC/CI error: Expected DDC/CI length bit
                                // This go away after the third attempt
                                error!("can't get_vcp_feature: {e}");
                                state = State::Waiting;
                                break;
                            }
                        };

                        let mon = Monitor {
                            name: display.info.model_name.clone().unwrap_or_default(),
                            brightness,
                        };

                        res.insert(display.info.id.clone(), mon);
                        displays.insert(display.info.id.clone(), Arc::new(Mutex::new(display)));
                    }

                    if let State::Waiting = state {
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
