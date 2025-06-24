#[macro_export]
macro_rules! icon_handle {
    ($name:literal) => {{
        let bytes = include_bytes!(concat!("../res/icons/", $name, ".svg"));
        cosmic::widget::icon::from_svg_bytes(bytes).symbolic(true)
    }};
}

#[macro_export]
macro_rules! icon {
    ($name:literal) => {{
        use $crate::icon_handle;

        cosmic::widget::icon::icon(icon_handle!($name))
    }};
}
#[macro_export]
macro_rules! icon_button {
    ($name:literal) => {{
        use $crate::icon_handle;
        cosmic::widget::button::icon(icon_handle!($name))
    }};
}

pub fn icon_high() -> cosmic::widget::icon::Handle {
    icon_handle!("cosmic-applet-battery-display-brightness-high-symbolic")
}

pub fn icon_medium() -> cosmic::widget::icon::Handle {
    icon_handle!("cosmic-applet-battery-display-brightness-medium-symbolic")
}
pub fn icon_low() -> cosmic::widget::icon::Handle {
    icon_handle!("cosmic-applet-battery-display-brightness-low-symbolic")
}
pub fn icon_off() -> cosmic::widget::icon::Handle {
    icon_handle!("cosmic-applet-battery-display-brightness-off-symbolic")
}