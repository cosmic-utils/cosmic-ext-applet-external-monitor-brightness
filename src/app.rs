use std::collections::HashMap;

use crate::config::{self, Config};
use crate::monitor::{DisplayId, EventToSub, Monitor};
use crate::{fl, monitor};
use cosmic::app::{Core, Task};
use cosmic::applet::{menu_button, padded_control};
use cosmic::cosmic_config::Config as CosmicConfig;
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::cosmic_theme::{THEME_MODE_ID, ThemeMode};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, Length, Limits, Subscription};
use cosmic::iced_runtime::core::window;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::widget::{
    Row, button, column, divider, horizontal_space, icon, mouse_area, row, slider, text, toggler,
};
use cosmic::{Element, iced_runtime};
use tokio::sync::watch::Sender;

pub(crate) const ID: &str = "io.github.cosmic_utils.cosmic-ext-applet-external-monitor-brightness";
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
    show_settings: bool,
    config: Config,
    config_handler: Option<CosmicConfig>,
    is_config_dirty: bool,
}

#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    SetScreenBrightness(String, f32),
    SetScreenCurve(String, f32),
    ChangeGlobalBrightness(f32),
    ToggleMinMaxBrightness(String),
    ThemeModeConfigChanged(ThemeMode),
    SetDarkMode(bool),
    ShowSettings(bool),
    Ready((HashMap<DisplayId, Monitor>, Sender<EventToSub>)),
    BrightnessWasUpdated(DisplayId, u16),
    UpdateMonitors(HashMap<DisplayId, Monitor>),
    Refresh,
}

impl Window {
    pub fn send(&self, e: EventToSub) {
        if let Some(sender) = &self.sender {
            sender.send(e).unwrap();

            // block_on(sender.send(e)).unwrap();
        }
    }

    fn sliders_view(&self) -> Vec<Element<Message>> {
        self.monitors
            .iter()
            .map(|(id, monitor)| {
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
                        .push(slider(
                            0..=100,
                            monitor.get_integer_brightness(),
                            move |brightness| {
                                Message::SetScreenBrightness(id.clone(), brightness as f32 / 100.0)
                            },
                        ))
                        .push(
                            text(format!("{:.0}%", monitor.get_curved_integer_brightness()))
                                .size(16)
                                .width(Length::Fixed(40.0)),
                        )
                        .spacing(12),
                )
                .into()
            })
            .collect()
    }

    fn settings_view(&self) -> Vec<Element<Message>> {
        if !self.show_settings {
            return Vec::with_capacity(0);
        }
        self.monitors
            .iter()
            .map(|(id, monitor)| {
                padded_control(
                    row()
                        .align_y(Alignment::Center)
                        .push(
                            button::icon(
                                icon::from_name(brightness_icon(monitor.brightness))
                                    .size(24)
                                    .symbolic(true),
                            )
                            .tooltip(&monitor.name),
                        )
                        .push(slider(
                            5..=20,
                            (monitor.gamma_curve * 10.0) as u32,
                            move |curve| Message::SetScreenCurve(id.clone(), curve as f32 / 10.0),
                        ))
                        .push(
                            text(format!("{:.1}", monitor.gamma_curve))
                                .size(16)
                                .width(Length::Fixed(40.0)),
                        )
                        .spacing(12),
                )
                .into()
            })
            .collect()
    }

    fn settings_collapsible_view(&self) -> Vec<Element<Message>> {
        let mut vec = Vec::with_capacity(2);
        // vec.push(padded_control(divider::horizontal::default()).into());
        if self.monitors.is_empty() {
            return vec;
        }
        vec.push(padded_control(divider::horizontal::default()).into());

        let dropdown_icon = if self.show_settings {
            "go-up-symbolic"
        } else {
            "go-down-symbolic"
        };

        let row = Row::new()
            .push(
                text::body(fl!("gamma-curve"))
                    .width(Length::Fill)
                    .height(Length::Fixed(24.0))
                    .align_y(Alignment::Center),
            )
            .push(
                cosmic::widget::container(icon::from_name(dropdown_icon).size(16).symbolic(true))
                    .center(Length::Fixed(24.0)),
            );
        vec.push(
            menu_button(row)
                .on_press(Message::ShowSettings(!self.show_settings))
                .into(),
        );

        vec
    }

    fn refresh_view(&self) -> Vec<Element<Message>> {
        let mut vec = Vec::with_capacity(2);

        let text = text::body(fl!("refresh"))
            .width(Length::Fill)
            .height(Length::Fixed(24.0))
            .align_y(Alignment::Center);
        vec.push(menu_button(text).on_press(Message::Refresh).into());

        vec
    }

    fn dark_mode_view(&self) -> Vec<Element<Message>> {
        let mut vec = Vec::with_capacity(2);
        if !self.monitors.is_empty() {
            vec.push(padded_control(divider::horizontal::default()).into());
        }

        vec.push(
            padded_control(
                mouse_area(
                    row()
                        .align_y(Alignment::Center)
                        .push(text(fl!("dark-mode")))
                        .push(horizontal_space())
                        .push(
                            toggler(self.theme_mode_config.is_dark).on_toggle(Message::SetDarkMode),
                        ),
                )
                .on_press(Message::SetDarkMode(!self.theme_mode_config.is_dark)),
            )
            .into(),
        );

        vec
    }

    fn apply_gamma_curves(&mut self) {
        for (monitor_id, monitor) in self.monitors.iter_mut() {
            if let Some((_id, gamma)) = self
                .config
                .gamma_curves
                .iter()
                .find(|(id, _)| id == monitor_id)
            {
                monitor.gamma_curve = *gamma;
            }
        }
    }

    fn reload_config(&mut self) {
        if let Some(config_handler) = &self.config_handler {
            if let Ok(c) = config::Config::get_entry(config_handler) {
                self.config = c;
                self.apply_gamma_curves();
            }
        }
    }

    fn save_config(&mut self) {
        if self.is_config_dirty {
            for (id, mon) in self.monitors.iter() {
                if let Some((_id, gamma)) = self
                    .config
                    .gamma_curves
                    .iter_mut()
                    .find(|(mon_id, _gamma)| mon_id == id)
                {
                    *gamma = mon.gamma_curve
                } else {
                    self.config.gamma_curves.push((id.clone(), mon.gamma_curve))
                }
            }
            if let Some(config_handler) = &self.config_handler {
                self.config
                    .write_entry(config_handler)
                    .unwrap_or_else(|e| error!("{e:?}"));
            }
            self.is_config_dirty = false;
        }
    }
}

impl cosmic::Application for Window {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = (Option<CosmicConfig>, Config);
    type Message = Message;
    const APP_ID: &'static str = ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let window = Window {
            core,
            config_handler: flags.0,
            config: flags.1,
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
            Message::Refresh => {
                self.send(EventToSub::Refresh { all: true });
                self.save_config();
                self.reload_config();
            }
            Message::UpdateMonitors(mon) => {
                self.monitors = mon;
                self.apply_gamma_curves();
            }
            Message::TogglePopup => {
                self.show_settings = false;
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    self.send(EventToSub::Refresh { all: false });
                    // self.refresh();

                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings =
                        self.core
                            .applet
                            .get_popup_settings(Id::RESERVED, new_id, None, None, None);
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(250.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                self.save_config();
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                    self.show_settings = false;
                }
            }
            Message::SetScreenBrightness(id, brightness) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    monitor.brightness = brightness;
                    let b = monitor.get_curved_integer_brightness();
                    self.send(EventToSub::Set(id, b));
                }
            }
            Message::ChangeGlobalBrightness(brightness) => {
                let ids: Vec<String> = self.monitors.keys().cloned().collect();
                for id in ids {
                    match self.monitors.get_mut(&id) {
                        Some(monitor) => {
                            // let b = (monitor.get_curved_brightness() + brightness).clamp(0.0, 1.0);
                            // monitor.set_curved_brightness(b);
                            monitor.change_brightness(brightness);
                            let b = monitor.get_curved_integer_brightness();
                            self.send(EventToSub::Set(id, b));
                        }
                        None => continue,
                    };
                }
            }
            Message::ToggleMinMaxBrightness(id) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    let new_val = match monitor.brightness {
                        x if x < 0.5 => 100,
                        _ => 0,
                    };
                    monitor.set_integer_brightness(new_val);
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
                self.apply_gamma_curves();
                self.sender.replace(sender);
            }
            Message::BrightnessWasUpdated(id, value) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    monitor.set_curved_integer_brightness(value);
                }
            }
            Message::SetScreenCurve(id, value) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    monitor.gamma_curve = value;
                    let b = monitor.get_curved_integer_brightness();
                    self.is_config_dirty = true;
                    self.send(EventToSub::Set(id, b));
                }
            }
            Message::ShowSettings(b) => self.show_settings = b,
        }
        Task::none()
    }

    fn view(&self) -> Element<Self::Message> {
        let btn = self
            .core
            .applet
            .icon_button(
                self.monitors
                    .values()
                    .next()
                    .map(|v| brightness_icon(v.brightness))
                    .unwrap_or(ICON_OFF),
            )
            .on_press(Message::TogglePopup);
        let btn = mouse_area(btn).on_scroll(|delta| {
            let change = match delta {
                cosmic::iced::mouse::ScrollDelta::Lines { x, y } => (x + y).signum() / 20.0,
                cosmic::iced::mouse::ScrollDelta::Pixels { y, .. } => y.signum() / 20.0,
            };
            Message::ChangeGlobalBrightness(change)
        });
        btn.into()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let col = column()
            .padding([8, 0])
            .extend(self.sliders_view())
            .extend(self.refresh_view())
            .extend(self.dark_mode_view())
            .extend(self.settings_collapsible_view())
            .extend(self.settings_view());
        self.core.applet.popup_container(col).into()
    }

    fn style(&self) -> Option<iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            self.core
                .watch_config(THEME_MODE_ID)
                .map(|u| Message::ThemeModeConfigChanged(u.config)),
            Subscription::run(monitor::sub),
            // Subscription::run(refresh_sub),
        ])
    }
}

fn brightness_icon(brightness: f32) -> &'static str {
    if brightness > 0.66 {
        ICON_HIGH
    } else if brightness > 0.33 {
        ICON_MEDIUM
    } else if brightness > 0.0 {
        ICON_LOW
    } else {
        ICON_OFF
    }
}
