mod config;
mod header_watch;
mod identity;
mod key_bindings;
mod settings;
mod theme;
mod ui;

use std::env;
use std::ffi::OsString;

use gtk::prelude::*;

use crate::config::AppConfig;
use crate::identity::{
    APPLICATION_ID, HEADER_HIDE_ACTION, HEADER_MINI_ACTION, HEADER_SHOW_ACTION,
    SETTINGS_RELOAD_ACTION,
};
use crate::settings::{HeaderMode, Settings};

#[cfg(debug_assertions)]
const USAGE: &str = "usage: zter [-s|--standalone|--new-tab] [--run <command>]\n       zter <-v|--version>\n       zter settings <apply|reload>\n       zter header <full|mini|hide|show>";
#[cfg(not(debug_assertions))]
const USAGE: &str = "usage: zter [-s|--standalone|--new-tab] [--run <command>]\n       zter <-v|--version>\n       zter settings <apply|reload>\n       zter header <full|mini|hide|show>\n       zter <hf|hm|hh>";
const VERSION_OUTPUT: &str = concat!("zter ", env!("CARGO_PKG_VERSION"));

fn main() -> gtk::glib::ExitCode {
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();
    match command_from_arguments(&arguments) {
        Ok(Command::Run { standalone, run }) => run_terminal(standalone, false, run),
        Ok(Command::NewTab { run }) => run_terminal(false, true, run),
        Ok(Command::SettingsApply) => apply_project_settings(),
        Ok(Command::SettingsReload) => reload_running_settings(),
        Ok(Command::HeaderMode(mode)) => set_header_mode(mode),
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

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Run {
        standalone: bool,
        run: Option<String>,
    },
    NewTab {
        run: Option<String>,
    },
    SettingsApply,
    SettingsReload,
    HeaderMode(HeaderMode),
    Version,
    Help,
}

fn command_from_arguments(arguments: &[OsString]) -> Result<Command, &'static str> {
    if let Some(index) = arguments.iter().position(|argument| argument == "--run") {
        return run_command_from_arguments(arguments, index);
    }
    match arguments {
        [] => Ok(Command::Run {
            standalone: false,
            run: None,
        }),
        [argument] if argument == "--standalone" || argument == "-s" => Ok(Command::Run {
            standalone: true,
            run: None,
        }),
        [argument] if argument == "--new-tab" => Ok(Command::NewTab { run: None }),
        [argument] if argument == "--version" || argument == "-v" => Ok(Command::Version),
        [argument] if argument == "--help" || argument == "-h" => Ok(Command::Help),
        [settings, apply] if settings == "settings" && apply == "apply" => {
            Ok(Command::SettingsApply)
        }
        [settings, reload] if settings == "settings" && reload == "reload" => {
            Ok(Command::SettingsReload)
        }
        [header, mode] if header == "header" && mode == "hide" => {
            Ok(Command::HeaderMode(HeaderMode::Hidden))
        }
        [header, mode] if header == "header" && (mode == "show" || mode == "full") => {
            Ok(Command::HeaderMode(HeaderMode::Full))
        }
        [header, mode] if header == "header" && mode == "mini" => {
            Ok(Command::HeaderMode(HeaderMode::Mini))
        }
        #[cfg(not(debug_assertions))]
        [alias] if alias == "hf" => Ok(Command::HeaderMode(HeaderMode::Full)),
        #[cfg(not(debug_assertions))]
        [alias] if alias == "hm" => Ok(Command::HeaderMode(HeaderMode::Mini)),
        #[cfg(not(debug_assertions))]
        [alias] if alias == "hh" => Ok(Command::HeaderMode(HeaderMode::Hidden)),
        _ => Err("unknown command"),
    }
}

/// Reads `--run <command>` at `index` and attaches it to the launch options
/// around it. The command is typed into a shell, so it must be one line of
/// printable UTF-8.
fn run_command_from_arguments(
    arguments: &[OsString],
    index: usize,
) -> Result<Command, &'static str> {
    let command = arguments
        .get(index + 1)
        .and_then(|command| command.to_str())
        .filter(|command| !command.trim().is_empty() && !command.chars().any(char::is_control))
        .ok_or("--run requires a single-line command")?
        .to_owned();
    let mut rest = arguments[..index].to_vec();
    rest.extend_from_slice(&arguments[index + 2..]);
    match command_from_arguments(&rest)? {
        Command::Run {
            standalone,
            run: None,
        } => Ok(Command::Run {
            standalone,
            run: Some(command),
        }),
        Command::NewTab { run: None } => Ok(Command::NewTab { run: Some(command) }),
        _ => Err("--run combines only with -s, --standalone or --new-tab, once"),
    }
}

fn application_flags(standalone: bool) -> gtk::gio::ApplicationFlags {
    if standalone {
        gtk::gio::ApplicationFlags::NON_UNIQUE | gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE
    } else {
        gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE
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

fn set_header_mode(mode: HeaderMode) -> gtk::glib::ExitCode {
    let path = match Settings::save_header_mode(mode) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("zter: could not update header mode: {error}");
            return gtk::glib::ExitCode::FAILURE;
        }
    };

    let application =
        gtk::gio::Application::new(Some(APPLICATION_ID), gtk::gio::ApplicationFlags::empty());
    match application.register(None::<&gtk::gio::Cancellable>) {
        Ok(()) if application.is_remote() => {
            let action = match mode {
                HeaderMode::Full => HEADER_SHOW_ACTION,
                HeaderMode::Mini => HEADER_MINI_ACTION,
                HeaderMode::Hidden => HEADER_HIDE_ACTION,
            };
            application.activate_action(action, None);
        }
        Ok(()) => {}
        Err(error) => eprintln!(
            "zter: warning: could not update a running application: {error}; the saved setting will apply on the next start"
        ),
    }

    println!("zter: header {}; updated {}", mode.as_str(), path.display());
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

fn run_terminal(standalone: bool, new_tab: bool, run: Option<String>) -> gtk::glib::ExitCode {
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

    application.add_main_option(
        "new-tab",
        gtk::glib::Char::from(0),
        gtk::glib::OptionFlags::NONE,
        gtk::glib::OptionArg::None,
        "Add a tab to the primary terminal, opening it if needed",
        None,
    );
    application.add_main_option(
        "run",
        gtk::glib::Char::from(0),
        gtk::glib::OptionFlags::NONE,
        gtk::glib::OptionArg::String,
        "Type a command into the shell of a new tab",
        Some("COMMAND"),
    );

    let activation_config = config.clone();
    application.connect_activate(move |application| {
        ui::activate(application, &activation_config, false, None);
    });

    application.connect_command_line(move |application, command_line| {
        let options = command_line.options_dict();
        let run = options.lookup::<String>("run").ok().flatten();
        ui::activate(
            application,
            &config,
            options.contains("new-tab"),
            run.as_deref(),
        );
        gtk::glib::ExitCode::SUCCESS
    });

    let _header_monitor = if standalone {
        match settings::settings_path()
            .map_err(|error| error.to_string())
            .and_then(|path| {
                header_watch::watch(&path, ui::apply_header_mode).map_err(|error| error.to_string())
            }) {
            Ok(monitor) => Some(monitor),
            Err(error) => {
                eprintln!("zter: could not watch header settings: {error}");
                None
            }
        }
    } else {
        None
    };

    let mut arguments = vec!["zter".to_owned()];
    if new_tab {
        arguments.push("--new-tab".to_owned());
    }
    // The = form keeps a command that starts with "-" as the option's value.
    if let Some(run) = run {
        arguments.push(format!("--run={run}"));
    }
    application.run_with_args(&arguments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_runs_the_terminal() {
        assert_eq!(
            command_from_arguments(&[]),
            Ok(Command::Run {
                standalone: false,
                run: None
            })
        );
    }

    #[test]
    fn long_standalone_option_runs_a_separate_instance() {
        let arguments = [OsString::from("--standalone")];

        assert_eq!(
            command_from_arguments(&arguments),
            Ok(Command::Run {
                standalone: true,
                run: None
            })
        );
    }

    #[test]
    fn short_standalone_option_runs_a_separate_instance() {
        let arguments = [OsString::from("-s")];

        assert_eq!(
            command_from_arguments(&arguments),
            Ok(Command::Run {
                standalone: true,
                run: None
            })
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
                | gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE
        );
    }

    #[test]
    fn normal_startup_uses_a_unique_gtk_application() {
        assert_eq!(
            application_flags(false),
            gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE
        );
    }

    #[test]
    fn new_tab_targets_the_primary_application_without_standalone() {
        assert_eq!(
            command_from_arguments(&[OsString::from("--new-tab")]),
            Ok(Command::NewTab { run: None })
        );
        for arguments in [["--standalone", "--new-tab"], ["--new-tab", "--standalone"]] {
            assert!(command_from_arguments(&arguments.map(OsString::from)).is_err());
        }
    }

    #[test]
    fn run_attaches_a_command_to_each_launch_option() {
        let parse = |arguments: &[&str]| {
            command_from_arguments(&arguments.iter().map(OsString::from).collect::<Vec<_>>())
        };
        let run = Some("npm run dev".to_owned());
        assert_eq!(
            parse(&["--run", "npm run dev"]),
            Ok(Command::Run {
                standalone: false,
                run: run.clone()
            })
        );
        for arguments in [
            ["-s", "--run", "npm run dev"],
            ["--run", "npm run dev", "--standalone"],
        ] {
            assert_eq!(
                parse(&arguments),
                Ok(Command::Run {
                    standalone: true,
                    run: run.clone()
                })
            );
        }
        assert_eq!(
            parse(&["--new-tab", "--run", "npm run dev"]),
            Ok(Command::NewTab { run })
        );
    }

    #[test]
    fn run_rejects_missing_multiline_repeated_or_unrelated_use() {
        for arguments in [
            &["--run"][..],
            &["--run", " "],
            &["--run", "codex\nrm -rf ~"],
            &["--run", "codex\u{1b}[2J"],
            &["--run", "a", "--run", "b"],
            &["-s", "--new-tab", "--run", "codex"],
            &["--version", "--run", "codex"],
            &["settings", "reload", "--run", "codex"],
        ] {
            let arguments: Vec<_> = arguments.iter().map(OsString::from).collect();
            assert!(command_from_arguments(&arguments).is_err(), "{arguments:?}");
        }
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
    fn header_hide_selects_the_hide_command() {
        let arguments = [OsString::from("header"), OsString::from("hide")];

        assert_eq!(
            command_from_arguments(&arguments),
            Ok(Command::HeaderMode(HeaderMode::Hidden))
        );
    }

    #[test]
    fn header_show_selects_the_show_command() {
        let arguments = [OsString::from("header"), OsString::from("show")];

        assert_eq!(
            command_from_arguments(&arguments),
            Ok(Command::HeaderMode(HeaderMode::Full))
        );
    }

    #[test]
    fn header_full_and_mini_select_their_modes() {
        for (argument, expected) in [("full", HeaderMode::Full), ("mini", HeaderMode::Mini)] {
            let arguments = [OsString::from("header"), OsString::from(argument)];
            assert_eq!(
                command_from_arguments(&arguments),
                Ok(Command::HeaderMode(expected))
            );
        }
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn release_header_aliases_select_their_modes() {
        for (alias, expected) in [
            ("hf", HeaderMode::Full),
            ("hm", HeaderMode::Mini),
            ("hh", HeaderMode::Hidden),
        ] {
            assert_eq!(
                command_from_arguments(&[OsString::from(alias)]),
                Ok(Command::HeaderMode(expected))
            );
            assert!(USAGE.contains(alias));
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn debug_build_rejects_release_header_aliases() {
        for alias in ["hf", "hm", "hh"] {
            assert_eq!(
                command_from_arguments(&[OsString::from(alias)]),
                Err("unknown command")
            );
            assert!(!USAGE.contains(alias));
        }
    }

    #[test]
    fn unknown_arguments_are_rejected() {
        let arguments = [OsString::from("settings"), OsString::from("unknown")];

        assert_eq!(command_from_arguments(&arguments), Err("unknown command"));
    }
}
