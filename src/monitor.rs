use std::{collections::HashMap, sync::{Arc, Mutex}};

use cosmic::iced::{subscription, Subscription};
use ddc_hi::{Ddc, Display};
use tokio::sync::mpsc;

use crate::window::Message;

const BRIGHTNESS_CODE: u8 = 0x10;

pub struct Monitor {
    pub display: Arc<Mutex<Display>>,
    pub brightness: u16,
    pub info: String,
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
            info: display.info.model_name.clone().unwrap_or_default(),
            display: Arc::new(Mutex::new(display)),
            brightness: 0,
        }
    }

    pub fn set_screen_brightness(&mut self, brightness: u16) {
        // self.brightness = brightness;
        // let _ = self
        //     .display
        //     .handle
        //     .set_vcp_feature(BRIGHTNESS_CODE, brightness);
    }

    pub fn update_brightness(&mut self) {
        // let start = std::time::Instant::now();

        // self.brightness = self
        //     .display
        //     .handle
        //     .get_vcp_feature(BRIGHTNESS_CODE)
        //     .map(|v| v.value())
        //     .unwrap_or_default();

        // let duration = start.elapsed();

        // log::debug!(
        //     "update_brightness for display {}: {:?}",
        //     self.display.info,
        //     duration
        // );
    }
}


pub enum EventToApp {
    Ready(mpsc::Sender<EventToSub>),
}

enum EventToSub {
    DoSomeWork,
    // ...
}


enum State {
    Starting,
    Ready(HashMap<DisplayId, Arc<Mutex<Display>>>),
}


pub fn sub() -> Subscription<Message> {

    struct Display;

    subscription::channel(std::any::TypeId::of::<Display>(), 100, |mut output| async move {
        loop {
            match &mut state {
                State::Starting => {
                    let (sender, receiver) = mpsc::channel(1);

                    output.send(Message::HandleReady(sender)).await;


                    // We are ready to receive messages
                    state = State::Ready(receiver);
                }
                State::Ready(receiver) => {
                    
                    while let Some(a) = receiver.
                }
            }
        }

    })

}



