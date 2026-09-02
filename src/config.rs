use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::settings::{Settings, SettingsError, TerminalPadding, Theme};

const FALLBACK_SHELL: &str = "/bin/sh";
pub(crate) const DEFAULT_BACKGROUND_IMAGE_SETTING: &str = "builtin";
pub const BACKGROUND_IMAGE_ENV: &str = "ZTER_BACKGROUND_IMAGE";
pub const WALLPAPER_ENV: &str = "ZTER_WALLPAPER";

#[derive(Clone, Debug, PartialEq)]
pub enum BackgroundImageSource {
    Default,
    File(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppConfig {
    shell: String,
    working_directory: String,
    background_image: Option<BackgroundImageSource>,
    theme: Theme,
    font_family: String,
    font_size: f64,
    terminal_padding: TerminalPadding,
    scrollback_lines: i64,
    background_image_opacity: f64,
    window_opacity: f64,
}

impl AppConfig {
    pub fn from_environment() -> Result<Self, ConfigError> {
        let settings = Settings::load_or_create().map_err(ConfigError::Settings)?;
        Self::from_values(
            settings,
            env::var_os("SHELL"),
            env::current_dir().map_err(ConfigError::CurrentDirectory)?,
            background_image_override(),
        )
    }

    fn from_values(
        settings: Settings,
        environment_shell: Option<OsString>,
        working_directory: PathBuf,
        background_image_override: Option<OsString>,
    ) -> Result<Self, ConfigError> {
        let shell = parse_shell(settings.shell(), environment_shell)?;
        let working_directory = path_to_string(working_directory)?;
        let background_image =
            parse_background_image(settings.background_image(), background_image_override);

        Ok(Self {
            shell,
            working_directory,
            background_image,
            theme: settings.theme(),
            font_family: settings.font_family().to_owned(),
            font_size: settings.font_size(),
            terminal_padding: settings.terminal_padding(),
            scrollback_lines: settings.scrollback_lines(),
            background_image_opacity: settings.background_image_opacity(),
            window_opacity: settings.window_opacity(),
        })
    }

    pub fn shell(&self) -> &str {
        &self.shell
    }

    pub fn working_directory(&self) -> &str {
        &self.working_directory
    }

    pub fn background_image(&self) -> Option<&BackgroundImageSource> {
        self.background_image.as_ref()
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

    pub fn background_image_opacity(&self) -> f64 {
        self.background_image_opacity
    }

    pub fn window_opacity(&self) -> f64 {
        self.window_opacity
    }
}

fn background_image_override() -> Option<OsString> {
    env::var_os(BACKGROUND_IMAGE_ENV).or_else(|| env::var_os(WALLPAPER_ENV))
}

#[derive(Debug)]
pub enum ConfigError {
    Settings(SettingsError),
    CurrentDirectory(std::io::Error),
    NonUnicodePath(PathBuf),
    NonUnicodeShell,
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
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Settings(error) => Some(error),
            Self::CurrentDirectory(error) => Some(error),
            Self::NonUnicodePath(_) | Self::NonUnicodeShell => None,
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

fn parse_background_image(
    configured_background_image: Option<&Path>,
    background_image_override: Option<OsString>,
) -> Option<BackgroundImageSource> {
    let background_image = match background_image_override {
        Some(value) if value.is_empty() => return None,
        Some(value) => Some(PathBuf::from(value)),
        None => configured_background_image.map(Path::to_owned),
    };

    let path = background_image?;
    if path == Path::new(DEFAULT_BACKGROUND_IMAGE_SETTING) {
        return Some(BackgroundImageSource::Default);
    }
    if !path.is_file() {
        eprintln!(
            "zter: warning: background image {} is not a file; using the default background image",
            path.display()
        );
        return Some(BackgroundImageSource::Default);
    }

    Some(BackgroundImageSource::File(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(
        environment_shell: Option<OsString>,
        background_image_override: Option<OsString>,
    ) -> Result<AppConfig, ConfigError> {
        AppConfig::from_values(
            Settings::defaults(),
            environment_shell,
            PathBuf::from("/tmp"),
            background_image_override,
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
    fn empty_background_image_override_disables_background_image() {
        let config = config(None, Some(OsString::new())).unwrap();

        assert_eq!(config.background_image(), None);
    }

    #[test]
    fn project_settings_use_the_default_background_image() {
        let config = config(None, None).unwrap();

        assert_eq!(
            config.background_image(),
            Some(&BackgroundImageSource::Default)
        );
    }

    #[test]
    fn default_background_image_override_is_supported() {
        let config = config(None, Some(OsString::from(DEFAULT_BACKGROUND_IMAGE_SETTING))).unwrap();

        assert_eq!(
            config.background_image(),
            Some(&BackgroundImageSource::Default)
        );
    }

    #[test]
    fn null_background_image_setting_disables_background_image() {
        let settings = customized_settings(serde_json::Value::Null, serde_json::Value::Null);
        let config = AppConfig::from_values(settings, None, PathBuf::from("/tmp"), None).unwrap();

        assert_eq!(config.background_image(), None);
    }

    #[test]
    fn missing_background_image_falls_back_to_the_default_background_image() {
        let missing = PathBuf::from("/zter-test/missing-background-image.png");

        let config = config(None, Some(missing.into_os_string())).unwrap();

        assert_eq!(
            config.background_image(),
            Some(&BackgroundImageSource::Default)
        );
    }

    #[test]
    fn terminal_defaults_are_exposed_to_the_ui() {
        let config = config(None, None).unwrap();

        assert_eq!(config.theme(), Theme::OneHalfDark);
        assert_eq!(config.font_family(), "Monospace");
        assert_eq!(config.font_size(), 12.0);
        assert_eq!(config.scrollback_lines(), 10_000);
        assert_eq!(config.background_image_opacity(), 0.1);
        assert_eq!(config.window_opacity(), 1.0);
    }

    #[test]
    fn terminal_padding_is_exposed_to_the_ui() {
        let config = config(None, None).unwrap();

        assert_eq!(
            config.terminal_padding(),
            TerminalPadding::new(16, 16, 16, 16)
        );
    }

    fn customized_settings(
        shell: serde_json::Value,
        background_image: serde_json::Value,
    ) -> Settings {
        serde_json::from_value(serde_json::json!({
            "schema_version": 3,
            "shell": shell,
            "background_image": background_image,
            "theme": "one-half-dark",
            "font_family": "Monospace",
            "font_size": 12.0,
            "padding_top": 0,
            "padding_right": 0,
            "padding_bottom": 0,
            "padding_left": 0,
            "scrollback_lines": 10000,
            "background_image_opacity": 0.1,
            "window_opacity": 1.0
        }))
        .unwrap()
    }
}
