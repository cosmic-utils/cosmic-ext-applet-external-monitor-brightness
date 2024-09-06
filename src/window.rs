use std::collections::HashMap;

use cosmic::app::{Command, Core};
use cosmic::applet::padded_control;
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::cosmic_theme::{ThemeMode, THEME_MODE_ID};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::wayland::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{Length, Limits, Subscription};
use cosmic::iced_runtime::core::window;
use cosmic::iced_style::application;
use cosmic::iced_widget::{row, Column};
use cosmic::widget::{button, container, divider, icon, slider, text};
use cosmic::{Element, Theme};
use cosmic_time::once_cell::sync::Lazy;
use cosmic_time::{anim, chain, id, Instant, Timeline};
// use tokio::sync::mpsc::Sender;
use crate::monitor::{DisplayId, EventToSub, Monitor};
use crate::{fl, monitor};
use tokio::sync::watch::Sender;

static SHOW_MEDIA_CONTROLS: Lazy<id::Toggler> = Lazy::new(id::Toggler::unique);

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
    timeline: Timeline,
    sender: Option<Sender<EventToSub>>,
}

#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    SetScreenBrightness(String, u16),
    ToggleMinMaxBrightness(String),
    ThemeModeConfigChanged(ThemeMode),
    SetDarkMode(chain::Toggler, bool),
    Frame(Instant),
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

    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let window = Window {
            core,
            ..Default::default()
        };

        (window, Command::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    self.send(EventToSub::Refresh);

                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    self.timeline = Timeline::new();
                    let mut popup_settings =
                        self.core
                            .applet
                            .get_popup_settings(Id::MAIN, new_id, None, None, None);
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
            Message::SetDarkMode(chain, dark) => {
                self.timeline.set_chain(chain).start();
                self.theme_mode_config.is_dark = dark;
                if let Ok(helper) = ThemeMode::config() {
                    _ = self.theme_mode_config.write_entry(&helper);
                }
            }
            Message::Frame(now) => self.timeline.now(now),
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
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        self.core
            .applet
            .icon_button(
                self.monitors
                    .values()
                    .next()
                    .map(|v| brightness_icon(v.brightness))
                    .unwrap_or(ICON_HIGH),
            )
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let mut content = Column::new();
        for (id, monitor) in &self.monitors {
            content = content.push(padded_control(
                row![
                    button::icon(
                        icon::from_name(brightness_icon(monitor.brightness))
                            .size(24)
                            .symbolic(true)
                    )
                    .tooltip(&monitor.name)
                    .on_press(Message::ToggleMinMaxBrightness(id.clone())),
                    slider(0..=100, monitor.brightness, move |brightness| {
                        Message::SetScreenBrightness(id.clone(), brightness)
                    }),
                    text(format!("{:.0}%", monitor.brightness))
                        .size(16)
                        .width(Length::Fixed(40.0))
                        .horizontal_alignment(Horizontal::Right)
                ]
                .spacing(12),
            ));
        }
        if !self.monitors.is_empty() {
            content = content.push(padded_control(divider::horizontal::default()));
        }
        content = content.push(
            container(
                anim!(
                    SHOW_MEDIA_CONTROLS,
                    &self.timeline,
                    Some(fl!("dark-mode").to_string()),
                    self.theme_mode_config.is_dark,
                    Message::SetDarkMode,
                )
                .text_size(14)
                .width(Length::Fill),
            )
            .padding([8, 24]),
        );

        self.core
            .applet
            .popup_container(content.padding([8, 0]))
            .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            self.core
                .watch_config(THEME_MODE_ID)
                .map(|u| Message::ThemeModeConfigChanged(u.config)),
            self.timeline
                .as_subscription()
                .map(|(_, now)| Message::Frame(now)),
            monitor::sub(),
        ])
    }

    fn style(&self) -> Option<<Theme as application::StyleSheet>::Style> {
        Some(cosmic::applet::style())
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
