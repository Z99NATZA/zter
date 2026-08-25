use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::settings::{Settings, SettingsError, TerminalPadding, Theme};

const FALLBACK_SHELL: &str = "/bin/sh";
pub const WALLPAPER_ENV: &str = "ZTER_WALLPAPER";

#[derive(Clone, Debug, PartialEq)]
pub struct AppConfig {
    shell: String,
    working_directory: String,
    wallpaper: Option<PathBuf>,
    theme: Theme,
    font_family: String,
    font_size: f64,
    terminal_padding: TerminalPadding,
    scrollback_lines: i64,
    wallpaper_opacity: f64,
}

impl AppConfig {
    pub fn from_environment() -> Result<Self, ConfigError> {
        let settings = Settings::load_or_create().map_err(ConfigError::Settings)?;
        Self::from_values(
            settings,
            env::var_os("SHELL"),
            env::current_dir().map_err(ConfigError::CurrentDirectory)?,
            env::var_os(WALLPAPER_ENV),
        )
    }

    fn from_values(
        settings: Settings,
        environment_shell: Option<OsString>,
        working_directory: PathBuf,
        wallpaper_override: Option<OsString>,
    ) -> Result<Self, ConfigError> {
        let shell = parse_shell(settings.shell(), environment_shell)?;
        let working_directory = path_to_string(working_directory)?;
        let wallpaper = parse_wallpaper(settings.wallpaper(), wallpaper_override)?;

        Ok(Self {
            shell,
            working_directory,
            wallpaper,
            theme: settings.theme(),
            font_family: settings.font_family().to_owned(),
            font_size: settings.font_size(),
            terminal_padding: settings.terminal_padding(),
            scrollback_lines: settings.scrollback_lines(),
            wallpaper_opacity: settings.wallpaper_opacity(),
        })
    }

    pub fn shell(&self) -> &str {
        &self.shell
    }

    pub fn working_directory(&self) -> &str {
        &self.working_directory
    }

    pub fn wallpaper(&self) -> Option<&Path> {
        self.wallpaper.as_deref()
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    pub fn font_family(&self) -> &str {
        &self.font_family
    }

    pub fn font_size(&self) -> f64 {
        self.font_size
    }

    pub fn terminal_padding(&self) -> TerminalPadding {
        self.terminal_padding
    }

    pub fn scrollback_lines(&self) -> i64 {
        self.scrollback_lines
    }

    pub fn wallpaper_opacity(&self) -> f64 {
        self.wallpaper_opacity
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Settings(SettingsError),
    CurrentDirectory(std::io::Error),
    NonUnicodePath(PathBuf),
    NonUnicodeShell,
    WallpaperNotFile(PathBuf),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Settings(error) => write!(formatter, "{error}"),
            Self::CurrentDirectory(error) => {
                write!(formatter, "cannot read the current directory: {error}")
            }
            Self::NonUnicodePath(path) => write!(
                formatter,
                "the working directory is not valid UTF-8: {}",
                path.display()
            ),
            Self::NonUnicodeShell => write!(formatter, "SHELL is not valid UTF-8"),
            Self::WallpaperNotFile(path) => write!(
                formatter,
                "the configured wallpaper does not point to a file: {}",
                path.display()
            ),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Settings(error) => Some(error),
            Self::CurrentDirectory(error) => Some(error),
            Self::NonUnicodePath(_) | Self::NonUnicodeShell | Self::WallpaperNotFile(_) => None,
        }
    }
}

fn parse_shell(
    configured_shell: Option<&str>,
    environment_shell: Option<OsString>,
) -> Result<String, ConfigError> {
    if let Some(shell) = configured_shell {
        return Ok(shell.to_owned());
    }

    match environment_shell {
        None => Ok(FALLBACK_SHELL.to_owned()),
        Some(shell) if shell.is_empty() => Ok(FALLBACK_SHELL.to_owned()),
        Some(shell) => shell
            .into_string()
            .map_err(|_| ConfigError::NonUnicodeShell),
    }
}

fn path_to_string(path: PathBuf) -> Result<String, ConfigError> {
    path.into_os_string()
        .into_string()
        .map_err(|path| ConfigError::NonUnicodePath(PathBuf::from(path)))
}

fn parse_wallpaper(
    configured_wallpaper: Option<&Path>,
    wallpaper_override: Option<OsString>,
) -> Result<Option<PathBuf>, ConfigError> {
    let wallpaper = match wallpaper_override {
        Some(value) if value.is_empty() => None,
        Some(value) => Some(PathBuf::from(value)),
        None => configured_wallpaper.map(Path::to_owned),
    };

    if let Some(path) = wallpaper.as_ref().filter(|path| !path.is_file()) {
        return Err(ConfigError::WallpaperNotFile(path.clone()));
    }

    Ok(wallpaper)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(
        environment_shell: Option<OsString>,
        wallpaper_override: Option<OsString>,
    ) -> Result<AppConfig, ConfigError> {
        AppConfig::from_values(
            Settings::defaults(),
            environment_shell,
            PathBuf::from("/tmp"),
            wallpaper_override,
        )
    }

    #[test]
    fn missing_shell_uses_posix_fallback() {
        let config = config(None, None).unwrap();

        assert_eq!(config.shell(), "/bin/sh");
    }

    #[test]
    fn environment_shell_is_used_when_settings_use_null() {
        let config = config(Some(OsString::from("/bin/fish")), None).unwrap();

        assert_eq!(config.shell(), "/bin/fish");
    }

    #[test]
    fn configured_shell_takes_precedence_over_the_environment() {
        let settings = customized_settings(serde_json::json!("/bin/zsh"), serde_json::Value::Null);
        let config = AppConfig::from_values(
            settings,
            Some(OsString::from("/bin/fish")),
            PathBuf::from("/tmp"),
            None,
        )
        .unwrap();

        assert_eq!(config.shell(), "/bin/zsh");
    }

    #[test]
    fn empty_wallpaper_override_disables_wallpaper() {
        let config = config(None, Some(OsString::new())).unwrap();

        assert_eq!(config.wallpaper(), None);
    }

    #[test]
    fn missing_wallpaper_is_rejected() {
        let missing = PathBuf::from("/zter-test/missing-wallpaper.png");

        let error = config(None, Some(missing.clone().into_os_string())).unwrap_err();

        assert!(matches!(error, ConfigError::WallpaperNotFile(path) if path == missing));
    }

    #[test]
    fn terminal_defaults_are_exposed_to_the_ui() {
        let config = config(None, None).unwrap();

        assert_eq!(config.theme(), Theme::OneHalfDark);
        assert_eq!(config.font_family(), "Monospace");
        assert_eq!(config.font_size(), 12.0);
        assert_eq!(config.scrollback_lines(), 10_000);
        assert_eq!(config.wallpaper_opacity(), 0.1);
    }

    #[test]
    fn terminal_padding_is_exposed_to_the_ui() {
        let config = config(None, None).unwrap();

        assert_eq!(config.terminal_padding(), TerminalPadding::default());
    }

    fn customized_settings(shell: serde_json::Value, wallpaper: serde_json::Value) -> Settings {
        serde_json::from_value(serde_json::json!({
            "schema_version": 2,
            "shell": shell,
            "wallpaper": wallpaper,
            "theme": "one-half-dark",
            "font_family": "Monospace",
            "font_size": 12.0,
            "padding_top": 0,
            "padding_right": 0,
            "padding_bottom": 0,
            "padding_left": 0,
            "scrollback_lines": 10000,
            "wallpaper_opacity": 0.1
        }))
        .unwrap()
    }
}
