mod config;
mod settings;
mod theme;
mod ui;

use std::env;
use std::ffi::OsString;

use gtk::prelude::*;

use crate::config::AppConfig;
use crate::settings::Settings;

const APPLICATION_ID: &str = "io.github.znnn.zter";
const USAGE: &str = "usage: zter [settings apply]";

fn main() -> gtk::glib::ExitCode {
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();
    match command_from_arguments(&arguments) {
        Ok(Command::Run) => run_terminal(),
        Ok(Command::SettingsApply) => apply_project_settings(),
        Ok(Command::Help) => {
            println!("{USAGE}");
            gtk::glib::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("zter: {error}\n{USAGE}");
            gtk::glib::ExitCode::FAILURE
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Run,
    SettingsApply,
    Help,
}

fn command_from_arguments(arguments: &[OsString]) -> Result<Command, &'static str> {
    match arguments {
        [] => Ok(Command::Run),
        [argument] if argument == "--help" || argument == "-h" => Ok(Command::Help),
        [settings, apply] if settings == "settings" && apply == "apply" => {
            Ok(Command::SettingsApply)
        }
        _ => Err("unknown command"),
    }
}

fn apply_project_settings() -> gtk::glib::ExitCode {
    let outcome = match Settings::apply_project() {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("zter: could not apply project settings: {error}");
            return gtk::glib::ExitCode::FAILURE;
        }
    };

    println!(
        "zter: applied project settings to {}",
        outcome.settings_path().display()
    );
    if let Some(backup_path) = outcome.backup_path() {
        println!("zter: previous settings saved to {}", backup_path.display());
    }

    gtk::glib::ExitCode::SUCCESS
}

fn run_terminal() -> gtk::glib::ExitCode {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_runs_the_terminal() {
        assert_eq!(command_from_arguments(&[]), Ok(Command::Run));
    }

    #[test]
    fn settings_apply_selects_the_apply_command() {
        let arguments = [OsString::from("settings"), OsString::from("apply")];

        assert_eq!(
            command_from_arguments(&arguments),
            Ok(Command::SettingsApply)
        );
    }

    #[test]
    fn unknown_arguments_are_rejected() {
        let arguments = [OsString::from("settings"), OsString::from("unknown")];

        assert_eq!(command_from_arguments(&arguments), Err("unknown command"));
    }
}
