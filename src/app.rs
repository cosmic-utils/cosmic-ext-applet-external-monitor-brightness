use std::collections::HashMap;
use std::time;

use crate::config::{self, Config};
use crate::monitor::{DisplayId, EventToSub, Monitor};
use crate::{fl, monitor};
use cosmic::app::{Core, Task};
use cosmic::applet::padded_control;
use cosmic::cosmic_config::Config as CosmicConfig;
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::cosmic_theme::{THEME_MODE_ID, ThemeMode};
use cosmic::iced::futures::{SinkExt, Stream};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, Length, Limits, Subscription, stream};
use cosmic::iced_runtime::core::window;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::widget::{
    button, column, divider, horizontal_space, icon, mouse_area, row, slider, text, toggler,
    tooltip,
};
use cosmic::{Element, iced_runtime};
use std::sync::mpsc::Receiver;
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
    is_config_dirty: bool,
    config: Config,
    config_handler: Option<CosmicConfig>,
    config_watch_rx: Option<Receiver<Config>>,
}

#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    SetScreenBrightness(String, f32),
    SetMonGammaMap(String, f32),
    ChangeGlobalBrightness(f32),
    // ToggleMinMaxBrightness(String),
    ThemeModeConfigChanged(ThemeMode),
    SetDarkMode(bool),
    ToggleMonSettings(String),
    Ready((HashMap<DisplayId, Monitor>, Sender<EventToSub>)),
    BrightnessWasUpdated(DisplayId, u16),
    Refresh,
}

impl Window {
    pub fn send(&self, e: EventToSub) {
        if let Some(sender) = &self.sender {
            sender.send(e).unwrap();

            // block_on(sender.send(e)).unwrap();
        }
    }

    fn sliders_view(&self) -> Option<Element<Message>> {
        if self.monitors.is_empty() {
            return None;
        }
        let mut column = column().padding(8.0);
        for (id, monitor) in self.monitors.iter() {
            column = column.extend(self.single_monitor_view(id, monitor));
        }
        Some(column.into())
    }

    fn single_monitor_view<'a>(
        &'a self,
        id: &'a String,
        monitor: &'a Monitor,
    ) -> Vec<Element<'a, Message>> {
        let mut elements = Vec::new();

        let Some(gamma_map) = self.config.get_gamma_map(id) else {
            return elements;
        };

        let mut root = row().spacing(0.0).padding(2.0);
        let mut left = column().spacing(8.0).padding(4.0);

        left = left.push(tooltip(
            icon::from_name(brightness_icon(monitor.brightness))
                .size(24)
                .symbolic(true),
            text(&monitor.name),
            tooltip::Position::Right,
        ));
        if monitor.settings_expanded {
            left = left.push(tooltip(
                icon::from_name("emblem-system-symbolic")
                    .size(24)
                    .symbolic(true),
                text(fl!("gamma-map")),
                tooltip::Position::Right,
            ))
        }
        let left = button::custom(left)
            .padding(0)
            .class(cosmic::style::Button::NavToggle)
            .on_press(Message::ToggleMonSettings(id.clone()));

        let mut right = column().spacing(8.0).padding(4.0);
        let main_slider = row()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(slider(
                0..=100,
                (monitor.brightness * 100.0) as u16,
                move |brightness| {
                    Message::SetScreenBrightness(id.clone(), brightness as f32 / 100.0)
                },
            ))
            .push(
                text(format!("{:.0}%", monitor.get_mapped_brightness(gamma_map)))
                    .size(16)
                    .width(Length::Fixed(35.0)),
            )
            .spacing(12);
        right = right.push(main_slider);
        if monitor.settings_expanded {
            let gamma_map_slider = row()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(slider(
                    5..=20,
                    (gamma_map * 10.0) as u16,
                    move |gamma_map| Message::SetMonGammaMap(id.clone(), gamma_map as f32 / 10.0),
                ))
                .push(
                    text(format!("{:.1}", gamma_map))
                        .size(16)
                        .width(Length::Fixed(35.0)),
                );
            right = right.push(gamma_map_slider);
        }

        root = root.push(left).push(right);
        elements.push(root.into());

        elements
    }

    fn dark_mode_view(&self) -> Vec<Element<Message>> {
        let mut vec = Vec::with_capacity(3);
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

    fn update_config(&mut self) {
        if let Some(rx) = &self.config_watch_rx {
            while let Ok(c) = rx.try_recv() {
                self.config = c;
            }
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
        let (config_tx, config_rx) = std::sync::mpsc::channel();
        let window = Window {
            core,
            config_handler: flags.0,
            config: flags.1,
            config_watch_rx: Some(config_rx),
            ..Default::default()
        };

        if let Some(c) = &window.config_handler {
            let watcher = c.watch(move |config, _strings| {
                let Ok(config) = Config::get_entry(config) else {
                    return;
                };
                config_tx.send(config).unwrap();
            });
            Box::leak(Box::new(watcher));
        }

        (window, Task::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        debug!("{:?}", message);

        match message {
            Message::Refresh => {
                // if let Some(last_dirty) = self.last_config_dirty {
                //     if time::Instant::now().duration_since(last_dirty)
                //         > time::Duration::from_secs(5)
                //     {
                //         for (id, mon) in self.monitors.iter() {
                //             if let Some((_id, gamma)) = self
                //                 .config
                //                 .gamma_curves
                //                 .iter_mut()
                //                 .find(|(mon_id, _gamma)| mon_id == id)
                //             {
                //                 *gamma = mon.gamma_curve
                //             } else {
                //                 self.config.gamma_curves.push((id.clone(), mon.gamma_curve))
                //             }
                //         }
                //         if let Some(config_handler) = &self.config_handler {
                //             self.config
                //                 .write_entry(config_handler)
                //                 .unwrap_or_else(|e| error!("{e:?}"));
                //         }
                //         self.last_config_dirty = None;
                //     }
                // }
            }
            Message::TogglePopup => {
                self.show_settings = false;
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    self.send(EventToSub::Refresh);

                    self.update_config();

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
                // save config to file
                if let Some(c) = &self.config_handler
                    && self.is_config_dirty
                {
                    self.is_config_dirty = false;
                    let _ = self.config.write_entry(c);
                }
                // collapse all monitor settings
                for (_id, mon) in self.monitors.iter_mut() {
                    mon.settings_expanded = false;
                }

                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                    self.show_settings = false;
                }
            }
            Message::SetScreenBrightness(id, brightness) => {
                if let Some(gamma_map) = self.config.get_gamma_map(&id) {
                    if let Some(monitor) = self.monitors.get_mut(&id) {
                        monitor.brightness = brightness;
                        let b = monitor.get_mapped_brightness(gamma_map);
                        self.send(EventToSub::Set(id, b));
                    }
                }
            }
            Message::ChangeGlobalBrightness(change) => {
                self.update_config();
                let ids: Vec<String> = self.monitors.keys().cloned().collect();
                for id in ids {
                    let Some(gamma_map) = self.config.get_gamma_map(&id) else {
                        continue;
                    };
                    match self.monitors.get_mut(&id) {
                        Some(monitor) => {
                            monitor.brightness = (monitor.brightness + change).clamp(0.0, 1.0);
                            let b = monitor.get_mapped_brightness(gamma_map);
                            self.send(EventToSub::Set(id, b));
                        }
                        None => continue,
                    };
                }
            }
            // Message::ToggleMinMaxBrightness(id) => {
            //     if let Some(monitor) = self.monitors.get_mut(&id) {
            //         let new_val = match monitor.brightness {
            //             x if x < 0.5 => 100,
            //             _ => 0,
            //         };
            //         monitor.brightness = new_val as f32 / 100.0;
            //         self.send(EventToSub::Set(id, new_val));
            //     }
            // }
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
                for (monitor_id, _monitor) in self.monitors.iter_mut() {
                    if self.config.get_monitor(monitor_id).is_none() {
                        self.config.monitors.push(config::Monitor {
                            id: monitor_id.clone(),
                            gamma_map: 1.0,
                        })
                    }
                }
                self.sender.replace(sender);
            }
            Message::BrightnessWasUpdated(id, value) => {
                if let Some(gamma_map) = self.config.get_gamma_map(&id) {
                    if let Some(monitor) = self.monitors.get_mut(&id) {
                        monitor.set_mapped_brightness(value, gamma_map);
                    }
                }
            }
            Message::SetMonGammaMap(id, value) => {
                if let Some(conf_mon) = self.config.monitors.iter_mut().find(|x| x.id == id) {
                    conf_mon.gamma_map = value;
                    self.is_config_dirty = true;
                }
            }
            Message::ToggleMonSettings(id) => {
                if let Some(mon) = self.monitors.get_mut(&id) {
                    mon.settings_expanded = !mon.settings_expanded;
                }
            }
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
            .push_maybe(self.sliders_view())
            .extend(self.dark_mode_view());
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
            Subscription::run(refresh_sub),
        ])
    }
}

fn refresh_sub() -> impl Stream<Item = Message> {
    stream::channel(10, |mut output| async move {
        loop {
            tokio::time::sleep(time::Duration::from_secs(10)).await;
            output.send(Message::Refresh).await.unwrap();
        }
    })
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
