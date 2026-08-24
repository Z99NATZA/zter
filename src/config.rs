use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

const FALLBACK_SHELL: &str = "/bin/sh";
pub const WALLPAPER_ENV: &str = "ZTER_WALLPAPER";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    shell: String,
    working_directory: String,
    wallpaper: Option<PathBuf>,
}

impl AppConfig {
    pub fn from_environment() -> Result<Self, ConfigError> {
        Self::from_values(
            env::var_os("SHELL"),
            env::current_dir().map_err(ConfigError::CurrentDirectory)?,
            env::var_os(WALLPAPER_ENV),
        )
    }

    fn from_values(
        shell: Option<OsString>,
        working_directory: PathBuf,
        wallpaper: Option<OsString>,
    ) -> Result<Self, ConfigError> {
        let shell = parse_shell(shell)?;
        let working_directory = path_to_string(working_directory)?;
        let wallpaper = parse_wallpaper(wallpaper)?;

        Ok(Self {
            shell,
            working_directory,
            wallpaper,
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
}

#[derive(Debug)]
pub enum ConfigError {
    CurrentDirectory(std::io::Error),
    NonUnicodePath(PathBuf),
    NonUnicodeShell,
    WallpaperNotFile(PathBuf),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
                "{WALLPAPER_ENV} does not point to a file: {}",
                path.display()
            ),
        }
    }
}

impl Error for ConfigError {}

fn parse_shell(shell: Option<OsString>) -> Result<String, ConfigError> {
    match shell {
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

fn parse_wallpaper(wallpaper: Option<OsString>) -> Result<Option<PathBuf>, ConfigError> {
    let Some(wallpaper) = wallpaper.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    let wallpaper = PathBuf::from(wallpaper);
    if !wallpaper.is_file() {
        return Err(ConfigError::WallpaperNotFile(wallpaper));
    }

    Ok(Some(wallpaper))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_shell_uses_posix_fallback() {
        let config = AppConfig::from_values(None, PathBuf::from("/tmp"), None).unwrap();

        assert_eq!(config.shell(), "/bin/sh");
    }

    #[test]
    fn configured_shell_is_preserved() {
        let config = AppConfig::from_values(
            Some(OsString::from("/bin/fish")),
            PathBuf::from("/tmp"),
            None,
        )
        .unwrap();

        assert_eq!(config.shell(), "/bin/fish");
    }

    #[test]
    fn empty_wallpaper_value_disables_wallpaper() {
        let config =
            AppConfig::from_values(None, PathBuf::from("/tmp"), Some(OsString::new())).unwrap();

        assert_eq!(config.wallpaper(), None);
    }

    #[test]
    fn missing_wallpaper_is_rejected() {
        let missing = PathBuf::from("/zter-test/missing-wallpaper.png");

        let error = AppConfig::from_values(
            None,
            PathBuf::from("/tmp"),
            Some(missing.clone().into_os_string()),
        )
        .unwrap_err();

        assert!(matches!(error, ConfigError::WallpaperNotFile(path) if path == missing));
    }
}
