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
use tokio::time::sleep;

use crate::app::Message;

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
    Fetching,
    Ready,
}

const MAX_WAITING: Duration = Duration::from_secs(4);
const DEFAULT_WAITING: Duration = Duration::from_millis(50);

pub fn sub() -> impl Stream<Item = Message> {
    stream::channel(100, |mut output| async move {
        let displays = Arc::new(Mutex::new(HashMap::new()));

        let mut rx = {
            let mut res = HashMap::new();

            for display in Display::enumerate() {
                let mon = Monitor {
                    name: display.info.model_name.clone().unwrap_or_default(),
                    brightness: 0,
                };

                res.insert(display.info.id.clone(), mon);
                displays
                    .lock()
                    .unwrap()
                    .insert(display.info.id.clone(), display);
            }

            let (tx, rx) = tokio::sync::mpsc::channel(100);
            output.send(Message::Ready((res, tx))).await.unwrap();
            rx
        };

        let mut state = State::Fetching;
        let mut duration = DEFAULT_WAITING;

        let mut request_buff = Vec::new();

        loop {
            match &mut state {
                State::Fetching => {
                    tokio::time::sleep(duration).await;

                    let (error, res) = {
                        let displays = displays.clone();

                        let j = tokio::task::spawn_blocking(move || {
                            let mut res = HashMap::new();

                            let mut displays = displays.lock().unwrap();

                            debug!("start enumerate");
                            for (id, display) in displays.iter_mut() {
                                match display.handle.get_vcp_feature(BRIGHTNESS_CODE) {
                                    Ok(v) => {
                                        res.insert(id.clone(), v.value());
                                    }
                                    Err(e) => {
                                        // on my machine, i get this error when starting the session
                                        // can't get_vcp_feature: DDC/CI error: Expected DDC/CI length bit
                                        // This go away after the third attempt
                                        error!("can't get_vcp_feature: {e}");
                                        continue;
                                    }
                                };
                            }
                            (res.len() != displays.len(), res)
                        });

                        j.await.unwrap()
                    };

                    output
                        .send(Message::BrightnessWasUpdated(res))
                        .await
                        .unwrap();

                    if error {
                        duration *= 2;
                        if duration > MAX_WAITING {
                            state = State::Ready;
                            duration = DEFAULT_WAITING;
                        }
                    } else {
                        duration = DEFAULT_WAITING;
                        state = State::Ready;
                    }
                }
                State::Ready => {
                    if let Some(e) = rx.recv().await {
                        request_buff.push(e);
                    }

                    while let Ok(e) = rx.try_recv() {
                        request_buff.push(e);
                    }

                    let mut set = HashMap::new();
                    let mut refresh = false;

                    for request in request_buff.drain(..) {
                        match request {
                            EventToSub::Refresh => refresh = true,
                            EventToSub::Set(id, value) => {
                                set.insert(id, value);
                            }
                        }
                    }

                    if refresh {
                        let displays = displays.clone();

                        let j = tokio::task::spawn_blocking(move || {
                            let mut res = HashMap::new();

                            for (id, display) in displays.lock().unwrap().iter_mut() {
                                match display.handle.get_vcp_feature(BRIGHTNESS_CODE) {
                                    Ok(v) => {
                                        res.insert(id.clone(), v.value());
                                    }
                                    Err(e) => {
                                        // on my machine, i get this error when starting the session
                                        // can't get_vcp_feature: DDC/CI error: Expected DDC/CI length bit
                                        // This go away after the third attempt
                                        error!("can't get_vcp_feature: {e}");
                                        continue;
                                    }
                                };
                            }
                            res
                        });

                        let res = j.await.unwrap();

                        output
                            .send(Message::BrightnessWasUpdated(res))
                            .await
                            .unwrap();
                    }

                    let displays = displays.clone();

                    let j = tokio::task::spawn_blocking(move || {
                        for (id, value) in set.drain() {
                            let mut displays = displays.lock().unwrap();

                            let display = displays.get_mut(&id).unwrap();

                            debug!("set {} to {}", id, value);
                            if let Err(err) = display.handle.set_vcp_feature(BRIGHTNESS_CODE, value)
                            {
                                error!("{:?}", err);
                            }
                        }
                    });
                    j.await.unwrap();
                    sleep(Duration::from_millis(50)).await;
                }
            }
        }
    })
}
