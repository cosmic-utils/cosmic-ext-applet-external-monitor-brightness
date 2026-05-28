use std::borrow::Cow;

use crate::app::{AppMsg, AppState, MonitorState};
use crate::fl;
use crate::icon::{icon_high, icon_low, icon_medium, icon_off};
use cosmic::Element;
use cosmic::applet::padded_control;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{
    button, column, container, divider, icon, mouse_area, row, slider, space::horizontal, text,
    toggler, tooltip,
};

impl AppState {
    pub fn applet_button_view(&self) -> Element<'_, AppMsg> {
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

    pub fn quick_settings_view(&self) -> Element<'_, AppMsg> {
        #[allow(dead_code)]
        fn toggle_settings<'a>(
            info: impl Into<Cow<'a, str>> + 'a,
            value: bool,
            f: impl Fn(bool) -> AppMsg + 'a,
        ) -> Element<'a, AppMsg> {
            row::with_capacity(3)
                .push(text(info))
                .push(horizontal())
                .push(toggler(value).on_toggle(f))
                .into()
        }

        column::with_capacity(1)
            .width(Length::Fill)
            .spacing(20)
            .padding(10)
            .push(button::text(fl!("refresh")).on_press(AppMsg::Refresh))
            .into()
    }

    pub fn popup_view(&self) -> Element<'_, AppMsg> {
        column::with_capacity(3)
            .padding(10)
            .push_maybe(self.monitors_view())
            .push_maybe(
                (!self.monitors.is_empty()).then(|| padded_control(divider::horizontal::default())),
            )
            .push(self.dark_mode_view())
            .into()
    }

    fn monitors_view(&self) -> Option<Element<'_, AppMsg>> {
        (!self.monitors.is_empty()).then(|| {
            column::with_capacity(2)
                .padding(8)
                .extend(
                    self.monitors
                        .iter()
                        .map(|(id, monitor)| self.monitor_view(id, monitor)),
                )
                .into()
        })
    }

    fn monitor_view<'a>(&self, id: &'a str, monitor: &'a MonitorState) -> Element<'a, AppMsg> {
        let gamma_map = self.config.get_gamma_map(id);

        row::with_capacity(2)
            .padding(2.0)
            .push(
                container(
                    mouse_area(
                        column::with_capacity(2)
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
                column::with_capacity(2)
                    .spacing(8.0)
                    .padding(4.0)
                    .push(
                        row::with_capacity(2)
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
                        row::with_capacity(2)
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

    //     column::with_capacity(1)
    //         .push(
    //             row::with_capacity(1)
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
    //             column::with_capacity(1).push(
    //                 row::with_capacity(1)
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

    fn dark_mode_view(&self) -> Element<'_, AppMsg> {
        padded_control(
            mouse_area(
                row::with_capacity(3)
                    .align_y(Alignment::Center)
                    .push(text(fl!("dark_mode")))
                    .push(horizontal())
                    .push(toggler(self.theme_mode_config.is_dark).on_toggle(AppMsg::SetDarkMode)),
            )
            .on_press(AppMsg::SetDarkMode(!self.theme_mode_config.is_dark)),
        )
        .into()
    }
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
