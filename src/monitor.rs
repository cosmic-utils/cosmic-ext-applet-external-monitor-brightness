use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use cosmic::iced::{futures::SinkExt, subscription, Subscription};
use ddc_hi::{Ddc, Display};
use log::error;
use tokio::sync::mpsc;

use crate::window::Message;

const BRIGHTNESS_CODE: u8 = 0x10;

pub type DisplayId = String;

#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub brightness: u16,
}

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
            for display in Display::enumerate() {
                let mon = Monitor {
                    name: display.info.model_name.clone().unwrap_or_default(),
                    brightness: 0,
                };

                res.insert(display.info.id.clone(), mon);
                displays.insert(display.info.id.clone(), display);
            }

            let (tx, mut rx) = mpsc::channel(1);

            output.send(Message::Ready((res, tx))).await;

            loop {
                match rx.recv().await {
                    Some(event) => match event {
                        EventToSub::Refresh => {
                            for (id, display) in &mut displays {
                                match display.handle.get_vcp_feature(BRIGHTNESS_CODE) {
                                    Ok(value) => {
                                        output
                                            .send(Message::BrightnessWasUpdated(
                                                id.clone(),
                                                value.value(),
                                            ))
                                            .await;
                                    }
                                    Err(err) => error!("{:?}", err),
                                }
                            }
                        }
                        EventToSub::Set(id, value) => {
                            if let Err(err) = displays
                                .get_mut(&id)
                                .unwrap()
                                .handle
                                .set_vcp_feature(BRIGHTNESS_CODE, value)
                            {
                                error!("{:?}", err);
                            }
                        }
                    },
                    None => todo!(),
                }
            }
        },
    )
}
