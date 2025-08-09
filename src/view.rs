use std::borrow::Cow;

use crate::app::{AppMsg, AppState, MonitorState};
use crate::fl;
use crate::icon::{icon_high, icon_low, icon_medium, icon_off};
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{
    button, column, container, horizontal_space, icon, mouse_area, row, slider, text, toggler,
    tooltip,
};

impl AppState {
    pub fn applet_button_view(&self) -> Element<AppMsg> {
        mouse_area(
            self.core
                .applet
                .icon_button_from_handle(
                    self.monitors
                        .values()
                        .next()
                        .map(|m| brightness_icon(m.slider_brightness))
                        .unwrap_or(icon_off()),
                )
                .on_press(AppMsg::TogglePopup),
        )
        .on_scroll(|delta| {
            let delta = match delta {
                cosmic::iced::mouse::ScrollDelta::Lines { x, y } => (x + y) / 20.0,
                cosmic::iced::mouse::ScrollDelta::Pixels { y, .. } => y / 300.0,
            };
            AppMsg::ChangeGlobalBrightness { delta }
        })
        .on_right_release(AppMsg::ToggleQuickSettings)
        .into()
    }

    pub fn quick_settings_view(&self) -> Element<AppMsg> {
        #[allow(dead_code)]
        fn toggle_settings<'a>(
            info: impl Into<Cow<'a, str>> + 'a,
            value: bool,
            f: impl Fn(bool) -> AppMsg + 'a,
        ) -> Element<'a, AppMsg> {
            row()
                .push(text(info))
                .push(horizontal_space())
                .push(toggler(value).on_toggle(f))
                .into()
        }

        column()
            .width(Length::Fill)
            .spacing(20)
            .padding(10)
            .push(button::text(fl!("refresh")).on_press(AppMsg::Refresh))
            .into()
    }

    pub fn popup_view(&self) -> Element<AppMsg> {
        column().padding(10).push(self.monitors_view()).into()
    }

    fn monitors_view(&self) -> Element<AppMsg> {
        if self.monitors.is_empty() {
            column().push(text(fl!("no_monitor"))).into()
        } else {
            column()
                .padding(8)
                .extend(
                    self.monitors
                        .iter()
                        .map(|(id, monitor)| self.monitor_view(id, monitor)),
                )
                .into()
        }
    }

    fn monitor_view<'a>(&self, id: &'a str, monitor: &'a MonitorState) -> Element<'a, AppMsg> {
        let gamma_map = self.config.get_gamma_map(id);

        row()
            .padding(2.0)
            .push(
                container(
                    mouse_area(
                        column()
                            .spacing(8.0)
                            .padding(4.0)
                            .push(tooltip(
                                icon::icon(brightness_icon(monitor.slider_brightness)).size(24),
                                text(&monitor.name),
                                tooltip::Position::Right,
                            ))
                            .push_maybe(monitor.settings_expanded.then(|| {
                                tooltip(
                                    icon::from_name("emblem-system-symbolic")
                                        .size(24)
                                        .symbolic(true),
                                    text(fl!("gamma_map")),
                                    tooltip::Position::Right,
                                )
                            })),
                    )
                    .on_press(AppMsg::ToggleMinMaxBrightness(id.to_string()))
                    .on_right_press(AppMsg::ToggleMonSettings(id.to_string()))
                    .on_scroll(|delta| {
                        let change = match delta {
                            cosmic::iced::mouse::ScrollDelta::Lines { x, y } => (x + y) / 20.0,
                            cosmic::iced::mouse::ScrollDelta::Pixels { y, .. } => y / 300.0,
                        };
                        AppMsg::SetScreenBrightness(
                            id.to_string(),
                            (monitor.slider_brightness + change).clamp(0.0, 1.0),
                        )
                    }),
                )
                .class(if monitor.settings_expanded {
                    cosmic::style::Container::Dropdown
                } else {
                    cosmic::style::Container::Transparent
                }),
            )
            .push(
                column()
                    .spacing(8.0)
                    .padding(4.0)
                    .push(
                        row()
                            .spacing(12)
                            .align_y(Alignment::Center)
                            .push(slider(
                                0..=100,
                                (monitor.slider_brightness * 100.0) as u16,
                                move |brightness| {
                                    AppMsg::SetScreenBrightness(
                                        id.to_string(),
                                        brightness as f32 / 100.0,
                                    )
                                },
                            ))
                            .push(
                                text(format!("{:.0}%", monitor.get_mapped_brightness(gamma_map)))
                                    .size(16)
                                    .width(Length::Fixed(35.0)),
                            ),
                    )
                    .push_maybe(monitor.settings_expanded.then(|| {
                        row()
                            .spacing(12)
                            .align_y(Alignment::Center)
                            .push(slider(
                                5..=20,
                                (gamma_map * 10.0) as u16,
                                move |gamma_map| {
                                    AppMsg::SetMonGammaMap(id.to_string(), gamma_map as f32 / 10.0)
                                },
                            ))
                            .push(
                                text(format!("{gamma_map:.1}"))
                                    .size(16)
                                    .width(Length::Fixed(35.0)),
                            )
                    })),
            )
            .into()
    }

    // fn monitor_view2<'a>(&self, id: &'a str, monitor: &'a MonitorState) -> Element<'a, AppMessage> {
    //     let gamma_map = self.config.get_gamma_map(id);

    //     column()
    //         .push(
    //             row()
    //                 .spacing(10)
    //                 .align_y(Alignment::Center)
    //                 .push(
    //                     mouse_area(tooltip(
    //                         icon::icon(brightness_icon(monitor.slider_brightness)).size(24),
    //                         text(&monitor.name),
    //                         tooltip::Position::Right,
    //                     ))
    //                     .on_press(AppMessage::ToggleMinMaxBrightness(id.to_string()))
    //                     .on_right_press(AppMessage::ToggleMonSettings(id.to_string()))
    //                     .on_scroll(|delta| {
    //                         let change = match delta {
    //                             cosmic::iced::mouse::ScrollDelta::Lines { x, y } => (x + y) / 20.0,
    //                             cosmic::iced::mouse::ScrollDelta::Pixels { y, .. } => y / 300.0,
    //                         };
    //                         AppMessage::SetScreenBrightness(
    //                             id.to_string(),
    //                             (monitor.slider_brightness + change).clamp(0.0, 1.0),
    //                         )
    //                     }),
    //                 )
    //                 .push(slider(
    //                     0..=100,
    //                     (monitor.slider_brightness * 100.0) as u16,
    //                     move |brightness| {
    //                         AppMessage::SetScreenBrightness(
    //                             id.to_string(),
    //                             brightness as f32 / 100.0,
    //                         )
    //                     },
    //                 ))
    //                 .push(
    //                     text(format!("{:.0}%", monitor.get_mapped_brightness(gamma_map)))
    //                         .size(16)
    //                         .width(Length::Fixed(35.0)),
    //                 ),
    //         )
    //         .push_maybe(monitor.settings_expanded.then(|| {
    //             column().push(
    //                 row()
    //                     .padding(10)
    //                     .spacing(12)
    //                     .align_y(Alignment::Center)
    //                     .push(tooltip(
    //                         icon::from_name("emblem-system-symbolic")
    //                             .size(24)
    //                             .symbolic(true),
    //                         text(fl!("gamma_map")),
    //                         tooltip::Position::Right,
    //                     ))
    //                     .push(slider(
    //                         5..=20,
    //                         (gamma_map * 10.0) as u16,
    //                         move |gamma_map| {
    //                             AppMessage::SetMonGammaMap(id.to_string(), gamma_map as f32 / 10.0)
    //                         },
    //                     ))
    //                     .push(
    //                         text(format!("{gamma_map:.1}"))
    //                             .size(16)
    //                             .width(Length::Fixed(35.0)),
    //                     ),
    //             )
    //         }))
    //         .into()
    // }
}

fn brightness_icon(brightness: f32) -> icon::Handle {
    if brightness > 0.66 {
        icon_high()
    } else if brightness > 0.33 {
        icon_medium()
    } else if brightness > 0.0 {
        icon_low()
    } else {
        icon_off()
    }
}
