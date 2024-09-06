use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use cosmic::iced::{futures::SinkExt, subscription, Subscription};
use ddc_hi::{Ddc, Display};

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

pub fn sub() -> Subscription<Message> {
    struct Worker;

    subscription::channel(
        std::any::TypeId::of::<Worker>(),
        100,
        |mut output| async move {
            let mut res = HashMap::new();

            let mut displays = HashMap::new();
            for mut display in Display::enumerate() {
                let mon = Monitor {
                    name: display.info.model_name.clone().unwrap_or_default(),
                    brightness: display
                        .handle
                        .get_vcp_feature(BRIGHTNESS_CODE)
                        .unwrap_or_default()
                        .value(),
                };

                res.insert(display.info.id.clone(), mon);
                displays.insert(display.info.id.clone(), Arc::new(Mutex::new(display)));
            }

            let (tx, mut rx) = tokio::sync::watch::channel(EventToSub::Refresh);
            rx.mark_unchanged();

            output.send(Message::Ready((res, tx))).await.unwrap();

            loop {
                rx.changed().await.unwrap();

                let last = rx.borrow_and_update().clone();
                match last {
                    EventToSub::Refresh => {
                        for (id, display) in &mut displays {
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
                    }
                }
            }
        },
    )
}
