use std::collections::HashMap;

use cosmic::app::{Core, Task};
use cosmic::applet::padded_control;
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::cosmic_theme::{ThemeMode, THEME_MODE_ID};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, Length, Limits, Subscription};
use cosmic::iced_runtime::core::window;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::widget::{button, column, divider, horizontal_space, icon, row, slider, text, toggler};
use cosmic::Element;
// use tokio::sync::mpsc::Sender;
use crate::monitor::{DisplayId, EventToSub, Monitor};
use crate::{fl, monitor};
use tokio::sync::watch::Sender;

const ID: &str = "io.github.maciekk64.CosmicExtAppletExternalMonitorBrightness";
const ICON_HIGH: &str = "cosmic-applet-battery-display-brightness-high-symbolic";
const ICON_MEDIUM: &str = "cosmic-applet-battery-display-brightness-medium-symbolic";
const ICON_LOW: &str = "cosmic-applet-battery-display-brightness-low-symbolic";
const ICON_OFF: &str = "cosmic-applet-battery-display-brightness-off-symbolic";

#[derive(Default)]
pub struct Window {
    core: Core,
    popup: Option<Id>,
    monitors: HashMap<DisplayId, Monitor>,
    theme_mode_config: ThemeMode,
    sender: Option<Sender<EventToSub>>,
}

#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    SetScreenBrightness(String, u16),
    ToggleMinMaxBrightness(String),
    ThemeModeConfigChanged(ThemeMode),
    SetDarkMode(bool),
    Ready((HashMap<DisplayId, Monitor>, Sender<EventToSub>)),
    BrightnessWasUpdated(DisplayId, u16),
}

impl Window {
    pub fn send(&self, e: EventToSub) {
        if let Some(sender) = &self.sender {
            sender.send(e).unwrap();

            // block_on(sender.send(e)).unwrap();
        }
    }
}

impl cosmic::Application for Window {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let window = Window {
            core,
            ..Default::default()
        };

        (window, Task::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        debug!("{:?}", message);

        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    self.send(EventToSub::Refresh);

                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings =
                        self.core
                            .applet
                            .get_popup_settings(Id::RESERVED, new_id, None, None, None);
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(372.0)
                        // .min_width(300.0)
                        // .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::SetScreenBrightness(id, brightness) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    monitor.brightness = brightness;
                }
                self.send(EventToSub::Set(id, brightness));
            }
            Message::ToggleMinMaxBrightness(id) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    let new_val = match monitor.brightness {
                        0 => 100,
                        _ => 0,
                    };
                    monitor.brightness = new_val;
                    self.send(EventToSub::Set(id, new_val));
                }
            }
            Message::ThemeModeConfigChanged(config) => {
                self.theme_mode_config = config;
            }
            Message::SetDarkMode(dark) => {
                self.theme_mode_config.is_dark = dark;
                if let Ok(helper) = ThemeMode::config() {
                    _ = self.theme_mode_config.write_entry(&helper);
                }
            }
            Message::Ready((mon, sender)) => {
                self.monitors = mon;
                self.sender.replace(sender);
            }
            Message::BrightnessWasUpdated(id, value) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    monitor.brightness = value;
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Self::Message> {
        self.core
            .applet
            .icon_button(
                self.monitors
                    .values()
                    .next()
                    .map(|v| brightness_icon(v.brightness))
                    .unwrap_or(ICON_OFF),
            )
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        self.core
            .applet
            .popup_container(
                column()
                    .padding([8, 0])
                    .extend(self.monitors.iter().map(|(id, monitor)| {
                        padded_control(
                            row()
                                .align_y(Alignment::Center)
                                .push(
                                    button::icon(
                                        icon::from_name(brightness_icon(monitor.brightness))
                                            .size(24)
                                            .symbolic(true),
                                    )
                                    .tooltip(&monitor.name)
                                    .on_press(Message::ToggleMinMaxBrightness(id.clone())),
                                )
                                .push(slider(0..=100, monitor.brightness, move |brightness| {
                                    Message::SetScreenBrightness(id.clone(), brightness)
                                }))
                                .push(
                                    text(format!("{:.0}%", monitor.brightness))
                                        .size(16)
                                        .width(Length::Fixed(40.0)),
                                )
                                .spacing(12),
                        )
                        .into()
                    }))
                    .push_maybe(if !self.monitors.is_empty() {
                        Some(padded_control(divider::horizontal::default()))
                    } else {
                        None
                    })
                    .push(padded_control(
                        row()
                            .align_y(Alignment::Center)
                            .push(text(fl!("dark-mode")))
                            .push(horizontal_space())
                            .push(
                                toggler(self.theme_mode_config.is_dark)
                                    .on_toggle(Message::SetDarkMode),
                            ),
                    )),
            )
            .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            self.core
                .watch_config(THEME_MODE_ID)
                .map(|u| Message::ThemeModeConfigChanged(u.config)),
            Subscription::run(monitor::sub),
        ])
    }
}

fn brightness_icon(brightness: u16) -> &'static str {
    if brightness > 66 {
        ICON_HIGH
    } else if brightness > 33 {
        ICON_MEDIUM
    } else if brightness > 0 {
        ICON_LOW
    } else {
        ICON_OFF
    }
}
