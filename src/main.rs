use crate::localize::localize;
use crate::window::Window;

#[macro_use]
extern crate log;

mod localize;
mod monitor;
mod window;

fn main() -> cosmic::iced::Result {
    env_logger::init();
    localize();
    cosmic::applet::run::<Window>(false, ())
}
