use std::collections::HashMap;

use crate::config::{self, Config, MonitorConfig};
use crate::monitor;
use crate::monitor::{DisplayId, EventToSub, MonitorInfo, ScreenBrightness};
use anyhow::anyhow;
use cosmic::app::{Core, Task};
use cosmic::cosmic_config::Config as CosmicConfig;
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::cosmic_theme::{THEME_MODE_ID, ThemeMode};
use cosmic::iced::window::Id;
use cosmic::iced::{Limits, Subscription};
use cosmic::iced_runtime::core::window;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::{Element, iced_runtime};
use tokio::sync::watch::Sender;

pub const APPID: &str = "io.github.cosmic_utils.cosmic-ext-applet-external-monitor-brightness";

#[derive(Debug, Clone)]
pub struct MonitorState {
    pub name: String,
    /// Between 0 and 1
    pub slider_brightness: f32,
    pub settings_expanded: bool,
}

pub fn get_mapped_brightness(slider_brightness: f32, gamma: f32) -> u16 {
    (slider_brightness.powf(gamma) * 100.0).round() as u16
}

pub fn get_slider_brightness(brightness: u16, gamma: f32) -> f32 {
    (brightness as f32 / 100.0).powf(1.0 / gamma)
}

impl MonitorState {
    pub fn get_mapped_brightness(&self, gamma: f32) -> u16 {
        get_mapped_brightness(self.slider_brightness, gamma)
    }

    pub fn set_slider_brightness(&mut self, brightness: u16, gamma: f32) {
        self.slider_brightness = get_slider_brightness(brightness, gamma)
    }
}

pub struct AppState {
    pub core: Core,
    popup: Option<Id>,
    pub monitors: HashMap<DisplayId, MonitorState>,
    pub theme_mode_config: ThemeMode,
    sender: Option<Sender<EventToSub>>,
    show_settings: bool,
    pub(crate) config: Config,
    config_handler: CosmicConfig,
}

#[derive(Clone, Debug)]
pub enum AppMessage {
    TogglePopup,
    PopupClosed(Id),

    ConfigChanged(Config),
    ThemeModeConfigChanged(ThemeMode),
    SetDarkMode(bool),

    SetScreenBrightness(DisplayId, f32),
    ToggleMinMaxBrightness(DisplayId),
    ChangeGlobalBrightness {
        delta: f32,
    },
    ToggleMonSettings(DisplayId),
    SetMonGammaMap(DisplayId, f32),

    /// Send from the subscription
    SubscriptionReady((HashMap<DisplayId, MonitorInfo>, Sender<EventToSub>)),
    /// Send from the subscription
    BrightnessWasUpdated(DisplayId, ScreenBrightness),
    // Refresh,
}

impl AppState {
    pub fn send(&self, e: EventToSub) {
        if let Some(sender) = &self.sender {
            sender.send(e).unwrap();

            // block_on(sender.send(e)).unwrap();
        }
    }

    fn update_monitor_config(&mut self, id: &str, f: impl Fn(&mut MonitorConfig)) {
        let mut monitors = self.config.monitors.clone();

        if let Some(monitor) = monitors.get_mut(id) {
            f(monitor);
        } else {
            let mut monitor = MonitorConfig::new();
            f(&mut monitor);
            monitors.insert(id.to_string(), monitor);
        }

        if let Err(e) = self.config.set_monitors(&self.config_handler, monitors) {
            error!("can't write config: {e}");
        }
    }
}

impl cosmic::Application for AppState {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = (Option<CosmicConfig>, Config);
    type Message = AppMessage;
    const APP_ID: &'static str = APPID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let window = AppState {
            core,
            config_handler: flags.0.expect("need to be able to write config"),
            config: flags.1,
            popup: None,
            monitors: HashMap::new(),
            theme_mode_config: ThemeMode::default(),
            sender: None,
            show_settings: false,
        };

        (window, Task::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<AppMessage> {
        Some(AppMessage::PopupClosed(id))
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        debug!("{:?}", message);

        match message {
            AppMessage::TogglePopup => {
                self.show_settings = false;
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
                        .max_width(250.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                };
            }
            AppMessage::PopupClosed(id) => {
                // collapse all monitor settings
                for (_id, mon) in self.monitors.iter_mut() {
                    mon.settings_expanded = false;
                }

                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                    self.show_settings = false;
                }
            }
            AppMessage::SetScreenBrightness(id, slider_brightness) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    monitor.slider_brightness = slider_brightness;
                    let gamma = self.config.get_gamma_map(&id);
                    let b = monitor.get_mapped_brightness(gamma);
                    self.send(EventToSub::Set(id, b));
                }
            }
            AppMessage::ChangeGlobalBrightness { delta } => {
                let mut vec = Vec::with_capacity(self.monitors.len());

                for (id, monitor) in self.monitors.iter_mut() {
                    monitor.slider_brightness = (monitor.slider_brightness + delta).clamp(0.0, 1.0);

                    let gamma = self.config.get_gamma_map(id);

                    let b = monitor.get_mapped_brightness(gamma);

                    vec.push(EventToSub::Set(id.clone(), b));
                }

                for e in vec {
                    self.send(e);
                }
            }
            AppMessage::ToggleMinMaxBrightness(id) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    let new_val = match monitor.slider_brightness {
                        x if x < 0.5 => 100,
                        _ => 0,
                    };
                    monitor.slider_brightness = new_val as f32 / 100.0;
                    self.send(EventToSub::Set(id, new_val));
                }
            }
            AppMessage::ThemeModeConfigChanged(config) => {
                self.theme_mode_config = config;
            }
            AppMessage::SetDarkMode(dark) => {
                fn set_theme_mode(mode: &ThemeMode) -> anyhow::Result<()> {
                    let home_dir = dirs::home_dir().ok_or(anyhow!("no home dir"))?;

                    let helper = cosmic::cosmic_config::Config::with_custom_path(
                        THEME_MODE_ID,
                        ThemeMode::VERSION,
                        home_dir.join(".config"),
                    )?;

                    mode.write_entry(&helper)?;

                    Ok(())
                }

                self.theme_mode_config.is_dark = dark;

                if let Err(e) = set_theme_mode(&self.theme_mode_config) {
                    error!("can't write theme mode {e}");
                }
            }
            AppMessage::SubscriptionReady((monitors, sender)) => {
                self.monitors = monitors
                    .into_iter()
                    .map(|(id, m)| {
                        (
                            id.clone(),
                            MonitorState {
                                name: m.name,
                                slider_brightness: get_slider_brightness(
                                    m.brightness,
                                    self.config.get_gamma_map(&id),
                                ),
                                settings_expanded: false,
                            },
                        )
                    })
                    .collect();

                self.sender.replace(sender);
            }
            AppMessage::BrightnessWasUpdated(id, brightness) => {
                if let Some(monitor) = self.monitors.get_mut(&id) {
                    monitor.set_slider_brightness(brightness, self.config.get_gamma_map(&id));
                }
            }
            AppMessage::SetMonGammaMap(id, gamma) => {
                if let Some(monitor) = self.monitors.get(&id) {
                    let b = monitor.get_mapped_brightness(gamma);
                    self.send(EventToSub::Set(id.clone(), b));
                }

                self.update_monitor_config(&id, |monitor| {
                    monitor.gamma_map = gamma;
                });
            }
            AppMessage::ToggleMonSettings(id) => {
                if let Some(mon) = self.monitors.get_mut(&id) {
                    mon.settings_expanded = !mon.settings_expanded;
                }
            }
            AppMessage::ConfigChanged(config) => self.config = config,
        }
        Task::none()
    }

    fn view(&self) -> Element<Self::Message> {
        self.applet_button_view()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        self.main_view()
    }

    fn style(&self) -> Option<iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            self.core
                .watch_config(THEME_MODE_ID)
                .map(|u| AppMessage::ThemeModeConfigChanged(u.config)),
            Subscription::run(monitor::sub),
            config::sub(),
            // Subscription::run(refresh_sub),
        ])
    }
}
