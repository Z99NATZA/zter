mod config;
mod theme;
mod ui;

use gtk::prelude::*;

use crate::config::AppConfig;

const APPLICATION_ID: &str = "io.github.znnn.zter";

fn main() -> gtk::glib::ExitCode {
    let config = match AppConfig::from_environment() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("zter: {error}");
            return gtk::glib::ExitCode::FAILURE;
        }
    };

    let application = gtk::Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    application.connect_activate(move |application| {
        ui::build(application, &config);
    });

    application.run()
}
