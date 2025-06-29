use cosmic::cosmic_config::{self, CosmicConfigEntry};

use crate::app::Window;
use crate::config::{CONFIG_VERSION, Config};
use crate::localize::localize;

#[macro_use]
extern crate tracing;

mod app;
mod config;
mod localize;
mod monitor;

fn setup_logs() {
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(format!(
        "warn,{}=warn",
        env!("CARGO_CRATE_NAME")
    )));

    if let Ok(journal_layer) = tracing_journald::layer() {
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .with(journal_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();
    }
}

fn main() -> cosmic::iced::Result {
    setup_logs();
    localize();

    let (config_handler, config) = match cosmic_config::Config::new(app::ID, CONFIG_VERSION) {
        Ok(config_handler) => {
            let config = match Config::get_entry(&config_handler) {
                Ok(ok) => ok,
                Err((errs, config)) => {
                    error!("errors loading config: {:?}", errs);
                    config
                }
            };
            (Some(config_handler), config)
        }
        Err(err) => {
            error!("failed to create config handler: {}", err);
            (None, Config::default())
        }
    };

    // config.write_entry(config_handler.as_ref().unwrap());

    cosmic::applet::run::<Window>((config_handler, config))
}
