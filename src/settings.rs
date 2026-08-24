use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const APPLICATION_DIRECTORY: &str = "zter";
const SETTINGS_FILE: &str = "settings.json";
const CURRENT_SCHEMA_VERSION: u32 = 1;
const PROJECT_SETTINGS_JSON: &str = include_str!("../config/settings.json");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    OneHalfDark,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    schema_version: u32,
    shell: Option<String>,
    wallpaper: Option<PathBuf>,
    theme: Theme,
    font_family: String,
    font_size: f64,
    scrollback_lines: i64,
    wallpaper_shade: f64,
}

impl Settings {
    pub fn load_or_create() -> Result<Self, SettingsError> {
        let path = settings_path()?;
        Self::load_or_create_at(&path)
    }

    pub fn apply_project() -> Result<ApplyOutcome, SettingsError> {
        let path = settings_path()?;
        Self::apply_project_at(&path)
    }

    fn load_or_create_at(path: &Path) -> Result<Self, SettingsError> {
        let default_value = project_settings_value()?;
        let defaults = settings_from_value(default_value.clone(), project_settings_path())?;

        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                write_settings(path, &defaults)?;
                return Ok(defaults);
            }
            Err(source) => {
                return Err(SettingsError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let mut value = parse_json(&source, path)?;
        let changed = merge_missing_keys(&mut value, &default_value, path)?;
        let settings = settings_from_value(value, path)?;

        if changed {
            write_settings(path, &settings)?;
        }

        Ok(settings)
    }

    fn apply_project_at(path: &Path) -> Result<ApplyOutcome, SettingsError> {
        let project_settings =
            settings_from_value(project_settings_value()?, project_settings_path())?;
        let backup_path = match fs::read(path) {
            Ok(existing_settings) => {
                let backup_path = path.with_file_name(format!("{SETTINGS_FILE}.bak"));
                write_bytes(&backup_path, &existing_settings)?;
                Some(backup_path)
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(SettingsError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };

        write_settings(path, &project_settings)?;

        Ok(ApplyOutcome {
            settings_path: path.to_owned(),
            backup_path,
        })
    }

    pub fn shell(&self) -> Option<&str> {
        self.shell.as_deref()
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

    pub fn scrollback_lines(&self) -> i64 {
        self.scrollback_lines
    }

    pub fn wallpaper_shade(&self) -> f64 {
        self.wallpaper_shade
    }

    #[cfg(test)]
    pub(crate) fn defaults() -> Self {
        settings_from_value(project_settings_value().unwrap(), project_settings_path()).unwrap()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ApplyOutcome {
    settings_path: PathBuf,
    backup_path: Option<PathBuf>,
}

impl ApplyOutcome {
    pub fn settings_path(&self) -> &Path {
        &self.settings_path
    }

    pub fn backup_path(&self) -> Option<&Path> {
        self.backup_path.as_deref()
    }
}

#[derive(Debug)]
pub enum SettingsError {
    ConfigDirectoryUnavailable,
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    Invalid {
        path: PathBuf,
        reason: String,
    },
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    Serialize(serde_json::Error),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigDirectoryUnavailable => write!(
                formatter,
                "cannot locate the config directory because XDG_CONFIG_HOME and HOME are unavailable"
            ),
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(formatter, "cannot parse {}: {source}", path.display())
            }
            Self::Invalid { path, reason } => {
                write!(
                    formatter,
                    "invalid settings in {}: {reason}",
                    path.display()
                )
            }
            Self::CreateDirectory { path, source } => write!(
                formatter,
                "cannot create settings directory {}: {source}",
                path.display()
            ),
            Self::Write { path, source } => {
                write!(formatter, "cannot write {}: {source}", path.display())
            }
            Self::Serialize(source) => write!(formatter, "cannot serialize settings: {source}"),
        }
    }
}

impl Error for SettingsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. }
            | Self::CreateDirectory { source, .. }
            | Self::Write { source, .. } => Some(source),
            Self::Parse { source, .. } | Self::Serialize(source) => Some(source),
            Self::ConfigDirectoryUnavailable | Self::Invalid { .. } => None,
        }
    }
}

fn settings_path() -> Result<PathBuf, SettingsError> {
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(config_home)
            .join(APPLICATION_DIRECTORY)
            .join(SETTINGS_FILE));
    }

    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .map(|home| {
            home.join(".config")
                .join(APPLICATION_DIRECTORY)
                .join(SETTINGS_FILE)
        })
        .ok_or(SettingsError::ConfigDirectoryUnavailable)
}

fn parse_json(source: &str, path: &Path) -> Result<Value, SettingsError> {
    serde_json::from_str(source).map_err(|source| SettingsError::Parse {
        path: path.to_owned(),
        source,
    })
}

fn project_settings_value() -> Result<Value, SettingsError> {
    parse_json(PROJECT_SETTINGS_JSON, project_settings_path())
}

fn project_settings_path() -> &'static Path {
    Path::new("config/settings.json")
}

fn merge_missing_keys(
    value: &mut Value,
    defaults: &Value,
    path: &Path,
) -> Result<bool, SettingsError> {
    let settings = value
        .as_object_mut()
        .ok_or_else(|| invalid(path, "the top-level value must be a JSON object"))?;
    let default_settings = defaults
        .as_object()
        .ok_or_else(|| invalid(path, "the embedded defaults must be a JSON object"))?;
    let mut changed = false;

    for (key, default_value) in default_settings {
        if !settings.contains_key(key) {
            settings.insert(key.clone(), default_value.clone());
            changed = true;
        }
    }

    Ok(changed)
}

fn settings_from_value(value: Value, path: &Path) -> Result<Settings, SettingsError> {
    let settings: Settings =
        serde_json::from_value(value).map_err(|source| SettingsError::Parse {
            path: path.to_owned(),
            source,
        })?;
    settings.validate(path)?;
    Ok(settings)
}

impl Settings {
    fn validate(&self, path: &Path) -> Result<(), SettingsError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(invalid(
                path,
                &format!(
                    "schema_version must be {CURRENT_SCHEMA_VERSION}, got {}",
                    self.schema_version
                ),
            ));
        }
        if self
            .shell
            .as_ref()
            .is_some_and(|shell| shell.trim().is_empty())
        {
            return Err(invalid(path, "shell must be null or a non-empty string"));
        }
        if self.font_family.trim().is_empty() {
            return Err(invalid(path, "font_family must not be empty"));
        }
        if !self.font_size.is_finite() || !(6.0..=72.0).contains(&self.font_size) {
            return Err(invalid(path, "font_size must be between 6 and 72"));
        }
        if !(0..=1_000_000).contains(&self.scrollback_lines) {
            return Err(invalid(
                path,
                "scrollback_lines must be between 0 and 1000000",
            ));
        }
        if !self.wallpaper_shade.is_finite() || !(0.0..=1.0).contains(&self.wallpaper_shade) {
            return Err(invalid(path, "wallpaper_shade must be between 0 and 1"));
        }

        Ok(())
    }
}

fn invalid(path: &Path, reason: &str) -> SettingsError {
    SettingsError::Invalid {
        path: path.to_owned(),
        reason: reason.to_owned(),
    }
}

fn write_settings(path: &Path, settings: &Settings) -> Result<(), SettingsError> {
    let mut serialized =
        serde_json::to_string_pretty(settings).map_err(SettingsError::Serialize)?;
    serialized.push('\n');

    write_bytes(path, serialized.as_bytes())
}

fn write_bytes(path: &Path, contents: &[u8]) -> Result<(), SettingsError> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid(path, "settings path has no parent directory"))?;
    fs::create_dir_all(parent).map_err(|source| SettingsError::CreateDirectory {
        path: parent.to_owned(),
        source,
    })?;

    let (temporary_path, mut temporary_file) = create_temporary_file(path)?;

    let write_result = (|| {
        temporary_file
            .write_all(contents)
            .and_then(|_| temporary_file.sync_all())
            .map_err(|source| SettingsError::Write {
                path: temporary_path.clone(),
                source,
            })?;
        fs::rename(&temporary_path, path).map_err(|source| SettingsError::Write {
            path: path.to_owned(),
            source,
        })
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }

    write_result
}

fn create_temporary_file(path: &Path) -> Result<(PathBuf, fs::File), SettingsError> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid(path, "settings path has no parent directory"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(SETTINGS_FILE);

    for attempt in 0..100 {
        let temporary_path = parent.join(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            attempt
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(SettingsError::Write {
                    path: temporary_path,
                    source,
                });
            }
        }
    }

    Err(invalid(path, "cannot allocate a temporary settings file"))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn embedded_defaults_contain_every_supported_key() {
        let value: Value = serde_json::from_str(PROJECT_SETTINGS_JSON).unwrap();
        let keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();

        assert_eq!(
            keys,
            [
                "font_family",
                "font_size",
                "schema_version",
                "scrollback_lines",
                "shell",
                "theme",
                "wallpaper",
                "wallpaper_shade",
            ]
        );
    }

    #[test]
    fn first_load_creates_a_complete_settings_file() {
        let directory = test_directory("create");
        let path = directory.join("zter/settings.json");

        let settings = Settings::load_or_create_at(&path).unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let defaults: Value = serde_json::from_str(PROJECT_SETTINGS_JSON).unwrap();

        assert_eq!(settings, Settings::defaults());
        assert_eq!(saved, defaults);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn missing_keys_are_added_without_discarding_user_values() {
        let directory = test_directory("merge");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, "{\"schema_version\":1,\"font_size\":16.0}").unwrap();

        let settings = Settings::load_or_create_at(&path).unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(settings.font_size(), 16.0);
        assert_eq!(saved["font_size"], 16.0);
        assert_eq!(saved.as_object().unwrap().len(), 8);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn malformed_file_is_not_overwritten() {
        let directory = test_directory("malformed");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let malformed = "{ this is not JSON";
        fs::write(&path, malformed).unwrap();

        assert!(Settings::load_or_create_at(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), malformed);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn invalid_value_is_rejected_without_overwriting_the_file() {
        let directory = test_directory("invalid");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let invalid = PROJECT_SETTINGS_JSON.replace("12.0", "100.0");
        fs::write(&path, &invalid).unwrap();

        assert!(matches!(
            Settings::load_or_create_at(&path),
            Err(SettingsError::Invalid { .. })
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn apply_project_backs_up_and_replaces_existing_settings() {
        let directory = test_directory("apply-existing");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let existing = b"{ malformed user settings }\n";
        fs::write(&path, existing).unwrap();

        let outcome = Settings::apply_project_at(&path).unwrap();
        let applied: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let project: Value = serde_json::from_str(PROJECT_SETTINGS_JSON).unwrap();
        let backup_path = directory.join("settings.json.bak");

        assert_eq!(outcome.settings_path(), path);
        assert_eq!(outcome.backup_path(), Some(backup_path.as_path()));
        assert_eq!(fs::read(&backup_path).unwrap(), existing);
        assert_eq!(applied, project);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn apply_project_without_existing_settings_does_not_create_a_backup() {
        let directory = test_directory("apply-new");
        let path = directory.join("zter/settings.json");

        let outcome = Settings::apply_project_at(&path).unwrap();

        assert_eq!(outcome.settings_path(), path);
        assert_eq!(outcome.backup_path(), None);
        assert!(path.is_file());
        assert!(!path.with_file_name("settings.json.bak").exists());
        fs::remove_dir_all(directory).unwrap();
    }

    fn test_directory(label: &str) -> PathBuf {
        let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        env::temp_dir().join(format!(
            "zter-settings-test-{}-{sequence}-{label}",
            std::process::id()
        ))
    }
}
