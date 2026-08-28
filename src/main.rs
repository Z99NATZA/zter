mod config;
mod identity;
mod settings;
mod theme;
mod ui;

use std::env;
use std::ffi::OsString;

use gtk::prelude::*;

use crate::config::AppConfig;
use crate::identity::{APPLICATION_ID, SETTINGS_RELOAD_ACTION};
use crate::settings::Settings;

const USAGE: &str = "usage: zter [-s|--standalone]\n       zter <-v|--version>\n       zter settings <apply|reload>";
const VERSION_OUTPUT: &str = concat!("zter ", env!("CARGO_PKG_VERSION"));

fn main() -> gtk::glib::ExitCode {
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();
    match command_from_arguments(&arguments) {
        Ok(Command::Run { standalone }) => run_terminal(standalone),
        Ok(Command::SettingsApply) => apply_project_settings(),
        Ok(Command::SettingsReload) => reload_running_settings(),
        Ok(Command::Version) => {
            println!("{VERSION_OUTPUT}");
            gtk::glib::ExitCode::SUCCESS
        }
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
    Run { standalone: bool },
    SettingsApply,
    SettingsReload,
    Version,
    Help,
}

fn command_from_arguments(arguments: &[OsString]) -> Result<Command, &'static str> {
    match arguments {
        [] => Ok(Command::Run { standalone: false }),
        [argument] if argument == "--standalone" || argument == "-s" => {
            Ok(Command::Run { standalone: true })
        }
        [argument] if argument == "--version" || argument == "-v" => Ok(Command::Version),
        [argument] if argument == "--help" || argument == "-h" => Ok(Command::Help),
        [settings, apply] if settings == "settings" && apply == "apply" => {
            Ok(Command::SettingsApply)
        }
        [settings, reload] if settings == "settings" && reload == "reload" => {
            Ok(Command::SettingsReload)
        }
        _ => Err("unknown command"),
    }
}

fn application_flags(standalone: bool) -> gtk::gio::ApplicationFlags {
    if standalone {
        gtk::gio::ApplicationFlags::NON_UNIQUE
    } else {
        gtk::gio::ApplicationFlags::empty()
    }
}

fn reload_running_settings() -> gtk::glib::ExitCode {
    let application =
        gtk::gio::Application::new(Some(APPLICATION_ID), gtk::gio::ApplicationFlags::empty());
    if let Err(error) = application.register(None::<&gtk::gio::Cancellable>) {
        eprintln!("zter: could not contact the running application: {error}");
        return gtk::glib::ExitCode::FAILURE;
    }
    if !application.is_remote() {
        println!("zter: no running application; settings will load on the next start");
        return gtk::glib::ExitCode::SUCCESS;
    }

    application.activate_action(SETTINGS_RELOAD_ACTION, None);
    println!("zter: requested settings reload");
    gtk::glib::ExitCode::SUCCESS
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

fn run_terminal(standalone: bool) -> gtk::glib::ExitCode {
    let config = match AppConfig::from_environment() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("zter: {error}");
            return gtk::glib::ExitCode::FAILURE;
        }
    };

    let application = gtk::Application::builder()
        .application_id(APPLICATION_ID)
        .flags(application_flags(standalone))
        .build();

    application.connect_activate(move |application| {
        ui::build(application, &config);
    });

    application.run_with_args(&["zter"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_runs_the_terminal() {
        assert_eq!(
            command_from_arguments(&[]),
            Ok(Command::Run { standalone: false })
        );
    }

    #[test]
    fn long_standalone_option_runs_a_separate_instance() {
        let arguments = [OsString::from("--standalone")];

        assert_eq!(
            command_from_arguments(&arguments),
            Ok(Command::Run { standalone: true })
        );
    }

    #[test]
    fn short_standalone_option_runs_a_separate_instance() {
        let arguments = [OsString::from("-s")];

        assert_eq!(
            command_from_arguments(&arguments),
            Ok(Command::Run { standalone: true })
        );
    }

    #[test]
    fn long_version_option_selects_the_version_command() {
        let arguments = [OsString::from("--version")];

        assert_eq!(command_from_arguments(&arguments), Ok(Command::Version));
    }

    #[test]
    fn short_version_option_selects_the_version_command() {
        let arguments = [OsString::from("-v")];

        assert_eq!(command_from_arguments(&arguments), Ok(Command::Version));
    }

    #[test]
    fn version_output_uses_cargo_package_metadata() {
        assert_eq!(
            VERSION_OUTPUT,
            format!("zter {}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn standalone_uses_a_non_unique_gtk_application() {
        assert_eq!(
            application_flags(true),
            gtk::gio::ApplicationFlags::NON_UNIQUE
        );
    }

    #[test]
    fn normal_startup_uses_a_unique_gtk_application() {
        assert_eq!(
            application_flags(false),
            gtk::gio::ApplicationFlags::empty()
        );
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
    fn settings_reload_selects_the_reload_command() {
        let arguments = [OsString::from("settings"), OsString::from("reload")];

        assert_eq!(
            command_from_arguments(&arguments),
            Ok(Command::SettingsReload)
        );
    }

    #[test]
    fn unknown_arguments_are_rejected() {
        let arguments = [OsString::from("settings"), OsString::from("unknown")];

        assert_eq!(command_from_arguments(&arguments), Err("unknown command"));
    }
}
