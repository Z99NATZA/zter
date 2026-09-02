use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::identity::SETTINGS_DIRECTORY;

const SETTINGS_FILE: &str = "settings.json";
const CURRENT_SCHEMA_VERSION: u32 = 3;
const DEFAULT_PADDING: u16 = 0;
pub(crate) const MAX_PADDING: u16 = 128;
pub(crate) const MIN_FONT_SIZE: f64 = 6.0;
pub(crate) const MAX_FONT_SIZE: f64 = 72.0;
pub(crate) const MAX_SCROLLBACK_LINES: i64 = 1_000_000;
pub(crate) const MAX_BACKGROUND_IMAGE_OPACITY: f64 = 0.6;
pub(crate) const MIN_WINDOW_OPACITY: f64 = 0.6;
pub(crate) const MAX_WINDOW_OPACITY: f64 = 1.0;
const PROJECT_SETTINGS_JSON: &str = include_str!("../config/settings.json");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    OneHalfDark,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalPadding {
    top: u16,
    right: u16,
    bottom: u16,
    left: u16,
}

impl TerminalPadding {
    pub(crate) fn new(top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn top(self) -> u16 {
        self.top
    }

    pub fn right(self) -> u16 {
        self.right
    }

    pub fn bottom(self) -> u16 {
        self.bottom
    }

    pub fn left(self) -> u16 {
        self.left
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    schema_version: u32,
    shell: Option<String>,
    background_image: Option<PathBuf>,
    theme: Theme,
    font_family: String,
    font_size: f64,
    #[serde(default = "default_padding", deserialize_with = "deserialize_padding")]
    padding_top: u16,
    #[serde(default = "default_padding", deserialize_with = "deserialize_padding")]
    padding_right: u16,
    #[serde(default = "default_padding", deserialize_with = "deserialize_padding")]
    padding_bottom: u16,
    #[serde(default = "default_padding", deserialize_with = "deserialize_padding")]
    padding_left: u16,
    scrollback_lines: i64,
    background_image_opacity: f64,
    window_opacity: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SettingsUpdate {
    pub shell: Option<String>,
    pub background_image: Option<PathBuf>,
    pub font_family: String,
    pub font_size: f64,
    pub terminal_padding: TerminalPadding,
    pub scrollback_lines: i64,
    pub background_image_opacity: f64,
    pub window_opacity: f64,
}

impl Settings {
    pub fn load_or_create() -> Result<Self, SettingsError> {
        match settings_path() {
            Ok(path) => Self::load_or_create_at(&path),
            Err(error) => {
                eprintln!("zter: warning: {error}; continuing with embedded settings defaults");
                settings_from_value(project_settings_value()?, project_settings_path())
            }
        }
    }

    pub fn apply_project() -> Result<ApplyOutcome, SettingsError> {
        let path = settings_path()?;
        Self::apply_project_at(&path)
    }

    pub(crate) fn save_user(&self) -> Result<PathBuf, SettingsError> {
        let path = settings_path()?;
        write_settings(&path, self)?;
        Ok(path)
    }

    pub(crate) fn apply_update(&mut self, update: SettingsUpdate) {
        let defaults = Self::defaults();
        self.shell = update
            .shell
            .map(|shell| shell.trim().to_owned())
            .filter(|shell| !shell.is_empty());
        self.background_image = update
            .background_image
            .filter(|background_image| !background_image.as_os_str().is_empty());
        self.font_family = nonempty_or(update.font_family, defaults.font_family);
        self.font_size = ranged_or(
            update.font_size,
            MIN_FONT_SIZE,
            MAX_FONT_SIZE,
            defaults.font_size,
        );
        self.padding_top = padding_or_default(update.terminal_padding.top(), defaults.padding_top);
        self.padding_right =
            padding_or_default(update.terminal_padding.right(), defaults.padding_right);
        self.padding_bottom =
            padding_or_default(update.terminal_padding.bottom(), defaults.padding_bottom);
        self.padding_left =
            padding_or_default(update.terminal_padding.left(), defaults.padding_left);
        self.scrollback_lines = if (0..=MAX_SCROLLBACK_LINES).contains(&update.scrollback_lines) {
            update.scrollback_lines
        } else {
            defaults.scrollback_lines
        };
        self.background_image_opacity = ranged_or(
            update.background_image_opacity,
            0.0,
            MAX_BACKGROUND_IMAGE_OPACITY,
            defaults.background_image_opacity,
        );
        self.window_opacity = ranged_or(
            update.window_opacity,
            MIN_WINDOW_OPACITY,
            MAX_WINDOW_OPACITY,
            defaults.window_opacity,
        );
    }

    fn load_or_create_at(path: &Path) -> Result<Self, SettingsError> {
        let default_value = project_settings_value()?;
        let defaults = settings_from_value(default_value.clone(), project_settings_path())?;

        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                if let Err(error) = write_settings(path, &defaults) {
                    warn_settings_fallback(path, &error.to_string());
                }
                return Ok(defaults);
            }
            Err(source) => {
                warn_settings_fallback(path, &format!("cannot read the file: {source}"));
                return Ok(defaults);
            }
        };
        let value = match parse_json(&source, path) {
            Ok(value) => value,
            Err(error) => {
                warn_settings_fallback(path, &error.to_string());
                return Ok(defaults);
            }
        };
        let (settings, changed) = resolve_user_settings(value, &default_value, path)?;

        if changed && let Err(error) = write_settings(path, &settings) {
            warn_settings_fallback(path, &error.to_string());
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

    pub fn background_image(&self) -> Option<&Path> {
        self.background_image.as_deref()
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
        TerminalPadding::new(
            self.padding_top,
            self.padding_right,
            self.padding_bottom,
            self.padding_left,
        )
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

    pub(crate) fn defaults() -> Self {
        settings_from_value(
            project_settings_value().expect("embedded settings must be valid JSON"),
            project_settings_path(),
        )
        .expect("embedded settings must satisfy the settings schema")
    }
}

fn nonempty_or(value: String, default: String) -> String {
    let value = value.trim();
    if value.is_empty() {
        default
    } else {
        value.to_owned()
    }
}

fn ranged_or(value: f64, minimum: f64, maximum: f64, default: f64) -> f64 {
    if value.is_finite() && (minimum..=maximum).contains(&value) {
        value
    } else {
        default
    }
}

fn padding_or_default(value: u16, default: u16) -> u16 {
    if value <= MAX_PADDING { value } else { default }
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
            .join(SETTINGS_DIRECTORY)
            .join(SETTINGS_FILE));
    }

    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .map(|home| {
            home.join(".config")
                .join(SETTINGS_DIRECTORY)
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

fn default_padding() -> u16 {
    DEFAULT_PADDING
}

fn deserialize_padding<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value
        .as_u64()
        .filter(|value| *value <= u64::from(MAX_PADDING))
        .map_or(DEFAULT_PADDING, |value| value as u16))
}

fn legacy_shade_to_opacity(shade: f64) -> f64 {
    (((1.0 - shade) * 100.0).round() / 100.0).min(0.6)
}

fn resolve_user_settings(
    value: Value,
    defaults: &Value,
    path: &Path,
) -> Result<(Settings, bool), SettingsError> {
    let default_settings = defaults
        .as_object()
        .ok_or_else(|| invalid(path, "the embedded defaults must be a JSON object"))?;
    let Some(mut user_settings) = value.as_object().cloned() else {
        warn_settings_fallback(path, "the top-level value is not a JSON object");
        return settings_from_value(defaults.clone(), project_settings_path())
            .map(|settings| (settings, false));
    };

    let mut changed = false;
    let mut has_invalid_keys = false;
    match user_settings.get("schema_version") {
        Some(value) if matches!(value.as_u64(), Some(1 | 2)) => {
            migrate_legacy_settings(&mut user_settings, &mut has_invalid_keys);
            changed = true;
        }
        Some(value) if value.as_u64() == Some(u64::from(CURRENT_SCHEMA_VERSION)) => {}
        None => changed = true,
        Some(Value::Null) => has_invalid_keys = true,
        Some(Value::String(value)) if value.trim().is_empty() => {
            has_invalid_keys = true;
        }
        Some(_) => {
            warn_settings_fallback(path, "schema_version is unsupported");
            return settings_from_value(defaults.clone(), project_settings_path())
                .map(|settings| (settings, false));
        }
    }

    let mut resolved = default_settings.clone();
    for key in default_settings.keys() {
        match user_settings.get(key) {
            Some(value) => match normalize_setting(key, value) {
                Some(value) => {
                    resolved.insert(key.clone(), value);
                }
                None => has_invalid_keys = true,
            },
            None => changed = true,
        }
    }
    for key in user_settings.keys() {
        if !default_settings.contains_key(key) {
            has_invalid_keys = true;
        }
    }

    let settings = settings_from_value(Value::Object(resolved), path)?;
    Ok((settings, changed && !has_invalid_keys))
}

fn migrate_legacy_settings(
    settings: &mut serde_json::Map<String, Value>,
    has_invalid_keys: &mut bool,
) {
    if !settings.contains_key("background_image")
        && let Some(wallpaper) = settings.get("wallpaper")
    {
        settings.insert("background_image".to_owned(), wallpaper.clone());
    }

    if !settings.contains_key("background_image_opacity") {
        if let Some(opacity) = settings.get("wallpaper_opacity") {
            settings.insert("background_image_opacity".to_owned(), opacity.clone());
        } else if let Some(shade) = settings.get("wallpaper_shade") {
            match shade
                .as_f64()
                .filter(|shade| shade.is_finite() && (0.0..=1.0).contains(shade))
            {
                Some(shade) => {
                    settings.insert(
                        "background_image_opacity".to_owned(),
                        Value::from(legacy_shade_to_opacity(shade)),
                    );
                }
                None => *has_invalid_keys = true,
            }
        }
    }

    settings.remove("wallpaper");
    settings.remove("wallpaper_opacity");
    settings.remove("wallpaper_shade");
    settings.insert(
        "schema_version".to_owned(),
        Value::from(CURRENT_SCHEMA_VERSION),
    );
}

fn normalize_setting(key: &str, value: &Value) -> Option<Value> {
    match key {
        "schema_version" => (value.as_u64() == Some(u64::from(CURRENT_SCHEMA_VERSION)))
            .then(|| Value::from(CURRENT_SCHEMA_VERSION)),
        "shell" | "background_image" => match value {
            Value::Null => Some(Value::Null),
            Value::String(value) if value.trim().is_empty() => Some(Value::Null),
            Value::String(_) => Some(value.clone()),
            _ => None,
        },
        "theme" => serde_json::from_value::<Theme>(value.clone())
            .ok()
            .map(|_| value.clone()),
        "font_family" => value
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .map(|_| value.clone()),
        "font_size" => value
            .as_f64()
            .filter(|value| value.is_finite() && (MIN_FONT_SIZE..=MAX_FONT_SIZE).contains(value))
            .map(|_| value.clone()),
        "padding_top" | "padding_right" | "padding_bottom" | "padding_left" => value
            .as_u64()
            .filter(|value| *value <= u64::from(MAX_PADDING))
            .map(Value::from),
        "scrollback_lines" => value
            .as_i64()
            .filter(|value| (0..=MAX_SCROLLBACK_LINES).contains(value))
            .map(Value::from),
        "background_image_opacity" => value
            .as_f64()
            .filter(|value| {
                value.is_finite() && (0.0..=MAX_BACKGROUND_IMAGE_OPACITY).contains(value)
            })
            .map(|_| value.clone()),
        "window_opacity" => value
            .as_f64()
            .filter(|value| {
                value.is_finite() && (MIN_WINDOW_OPACITY..=MAX_WINDOW_OPACITY).contains(value)
            })
            .map(|_| value.clone()),
        _ => None,
    }
}

fn warn_settings_fallback(path: &Path, reason: &str) {
    eprintln!(
        "zter: warning: could not use all settings from {}: {reason}; continuing with safe defaults",
        path.display()
    );
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
        if !self.font_size.is_finite() || !(MIN_FONT_SIZE..=MAX_FONT_SIZE).contains(&self.font_size)
        {
            return Err(invalid(path, "font_size must be between 6 and 72"));
        }
        if !(0..=MAX_SCROLLBACK_LINES).contains(&self.scrollback_lines) {
            return Err(invalid(
                path,
                "scrollback_lines must be between 0 and 1000000",
            ));
        }
        if !self.background_image_opacity.is_finite()
            || !(0.0..=MAX_BACKGROUND_IMAGE_OPACITY).contains(&self.background_image_opacity)
        {
            return Err(invalid(
                path,
                "background_image_opacity must be between 0 and 0.6",
            ));
        }
        if !self.window_opacity.is_finite()
            || !(MIN_WINDOW_OPACITY..=MAX_WINDOW_OPACITY).contains(&self.window_opacity)
        {
            return Err(invalid(path, "window_opacity must be between 0.6 and 1.0"));
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
                "background_image",
                "background_image_opacity",
                "font_family",
                "font_size",
                "padding_bottom",
                "padding_left",
                "padding_right",
                "padding_top",
                "schema_version",
                "scrollback_lines",
                "shell",
                "theme",
                "window_opacity",
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
        assert_eq!(saved.as_object().unwrap().len(), 13);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn padding_accepts_each_supported_boundary() {
        let mut value = project_settings_value().unwrap();
        let settings = value.as_object_mut().unwrap();
        settings.insert("padding_top".to_owned(), Value::from(0));
        settings.insert("padding_right".to_owned(), Value::from(MAX_PADDING));
        settings.insert("padding_bottom".to_owned(), Value::from(24));
        settings.insert("padding_left".to_owned(), Value::from(8));

        let settings = settings_from_value(value, project_settings_path()).unwrap();

        assert_eq!(
            settings.terminal_padding(),
            TerminalPadding::new(0, MAX_PADDING, 24, 8)
        );
    }

    #[test]
    fn invalid_padding_fields_fall_back_independently() {
        let mut value = project_settings_value().unwrap();
        let settings = value.as_object_mut().unwrap();
        settings.insert("padding_top".to_owned(), Value::from(12));
        settings.insert("padding_right".to_owned(), Value::from(129));
        settings.insert("padding_bottom".to_owned(), Value::from("8"));
        settings.insert("padding_left".to_owned(), Value::from(4.5));

        let settings = settings_from_value(value, project_settings_path()).unwrap();

        assert_eq!(
            settings.terminal_padding(),
            TerminalPadding::new(12, 0, 0, 0)
        );
    }

    #[test]
    fn invalid_padding_does_not_prevent_loading_or_overwrite_the_file() {
        let directory = test_directory("invalid-padding");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let mut value = project_settings_value().unwrap();
        let values = value.as_object_mut().unwrap();
        values.insert("padding_top".to_owned(), Value::from(-1));
        values.insert("padding_right".to_owned(), Value::Null);
        let invalid = format!("{}\n", serde_json::to_string_pretty(&value).unwrap());
        fs::write(&path, &invalid).unwrap();

        let settings = Settings::load_or_create_at(&path).unwrap();

        assert_eq!(
            settings.terminal_padding(),
            Settings::defaults().terminal_padding()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn null_and_empty_values_fall_back_without_discarding_other_keys() {
        let directory = test_directory("null-empty");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let mut value = project_settings_value().unwrap();
        let values = value.as_object_mut().unwrap();
        values.insert("shell".to_owned(), Value::from(""));
        values.insert("background_image".to_owned(), Value::from(""));
        values.insert("theme".to_owned(), Value::Null);
        values.insert("font_family".to_owned(), Value::from(""));
        values.insert("font_size".to_owned(), Value::from(""));
        values.insert("padding_top".to_owned(), Value::Null);
        values.insert("padding_right".to_owned(), Value::from(""));
        values.insert("padding_bottom".to_owned(), Value::from(999));
        values.insert("padding_left".to_owned(), Value::from(4.5));
        values.insert("scrollback_lines".to_owned(), Value::from(""));
        values.insert("background_image_opacity".to_owned(), Value::Null);
        values.insert("window_opacity".to_owned(), Value::Null);
        let source = format!("{}\n", serde_json::to_string_pretty(&value).unwrap());
        fs::write(&path, &source).unwrap();

        let settings = Settings::load_or_create_at(&path).unwrap();
        let defaults = Settings::defaults();

        assert_eq!(settings.shell(), None);
        assert_eq!(settings.background_image(), None);
        assert_eq!(settings.theme(), defaults.theme());
        assert_eq!(settings.font_family(), defaults.font_family());
        assert_eq!(settings.font_size(), defaults.font_size());
        assert_eq!(settings.terminal_padding(), defaults.terminal_padding());
        assert_eq!(settings.scrollback_lines(), defaults.scrollback_lines());
        assert_eq!(
            settings.background_image_opacity(),
            defaults.background_image_opacity()
        );
        assert_eq!(settings.window_opacity(), defaults.window_opacity());
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn schema_one_shade_migrates_to_equivalent_background_image_opacity() {
        let directory = test_directory("shade-migration");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            &path,
            "{\"schema_version\":1,\"font_size\":16.0,\"wallpaper_shade\":0.8}",
        )
        .unwrap();

        let settings = Settings::load_or_create_at(&path).unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(settings.background_image_opacity(), 0.2);
        assert_eq!(saved["schema_version"], 3);
        assert!(saved.get("wallpaper_shade").is_none());
        assert!(saved.get("wallpaper_opacity").is_none());
        assert_eq!(saved["background_image_opacity"], 0.2);
        assert_eq!(saved["window_opacity"], 1.0);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn schema_two_wallpaper_keys_migrate_to_background_image_keys() {
        let directory = test_directory("schema-two-migration");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            &path,
            "{\"schema_version\":2,\"wallpaper\":\"/tmp/bg.png\",\"wallpaper_opacity\":0.25}",
        )
        .unwrap();

        let settings = Settings::load_or_create_at(&path).unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(settings.background_image(), Some(Path::new("/tmp/bg.png")));
        assert_eq!(settings.background_image_opacity(), 0.25);
        assert_eq!(settings.window_opacity(), 1.0);
        assert_eq!(saved["schema_version"], 3);
        assert!(saved.get("wallpaper").is_none());
        assert!(saved.get("wallpaper_opacity").is_none());
        assert_eq!(saved["background_image"], "/tmp/bg.png");
        assert_eq!(saved["background_image_opacity"], 0.25);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn schema_one_migration_caps_opacity_at_the_supported_maximum() {
        assert_eq!(legacy_shade_to_opacity(0.0), 0.6);
    }

    #[test]
    fn malformed_file_is_not_overwritten() {
        let directory = test_directory("malformed");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let malformed = "{ this is not JSON";
        fs::write(&path, malformed).unwrap();

        assert_eq!(
            Settings::load_or_create_at(&path).unwrap(),
            Settings::defaults()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), malformed);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn non_utf8_file_uses_defaults_without_overwriting_the_file() {
        let directory = test_directory("non-utf8");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let source = b"{\"font_size\":\xff}";
        fs::write(&path, source).unwrap();

        assert_eq!(
            Settings::load_or_create_at(&path).unwrap(),
            Settings::defaults()
        );
        assert_eq!(fs::read(&path).unwrap(), source);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn invalid_value_uses_its_default_without_discarding_valid_values() {
        let directory = test_directory("invalid");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let mut value = project_settings_value().unwrap();
        let values = value.as_object_mut().unwrap();
        values.insert("shell".to_owned(), Value::from("/bin/zsh"));
        values.insert("font_size".to_owned(), Value::from(100.0));
        let invalid = format!("{}\n", serde_json::to_string_pretty(&value).unwrap());
        fs::write(&path, &invalid).unwrap();

        let settings = Settings::load_or_create_at(&path).unwrap();

        assert_eq!(settings.shell(), Some("/bin/zsh"));
        assert_eq!(settings.font_size(), Settings::defaults().font_size());
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn background_image_opacity_above_the_supported_maximum_uses_its_default() {
        let directory = test_directory("invalid-opacity");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let mut value = project_settings_value().unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("background_image_opacity".to_owned(), Value::from(0.7));
        let invalid = format!("{}\n", serde_json::to_string_pretty(&value).unwrap());
        fs::write(&path, &invalid).unwrap();

        let settings = Settings::load_or_create_at(&path).unwrap();

        assert_eq!(
            settings.background_image_opacity(),
            Settings::defaults().background_image_opacity()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn window_opacity_outside_supported_range_uses_its_default() {
        let directory = test_directory("invalid-window-opacity");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let mut value = project_settings_value().unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("window_opacity".to_owned(), Value::from(0.5));
        let invalid = format!("{}\n", serde_json::to_string_pretty(&value).unwrap());
        fs::write(&path, &invalid).unwrap();

        let settings = Settings::load_or_create_at(&path).unwrap();

        assert_eq!(
            settings.window_opacity(),
            Settings::defaults().window_opacity()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unsupported_schema_uses_defaults_without_overwriting_the_file() {
        let directory = test_directory("unsupported-schema");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let source =
            PROJECT_SETTINGS_JSON.replace("\"schema_version\": 3", "\"schema_version\": 99");
        fs::write(&path, &source).unwrap();

        assert_eq!(
            Settings::load_or_create_at(&path).unwrap(),
            Settings::defaults()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unknown_keys_are_ignored_without_discarding_known_values() {
        let directory = test_directory("unknown-key");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let mut value = project_settings_value().unwrap();
        let values = value.as_object_mut().unwrap();
        values.insert("font_size".to_owned(), Value::from(18.0));
        values.insert("future_setting".to_owned(), Value::Bool(true));
        let source = format!("{}\n", serde_json::to_string_pretty(&value).unwrap());
        fs::write(&path, &source).unwrap();

        let settings = Settings::load_or_create_at(&path).unwrap();

        assert_eq!(settings.font_size(), 18.0);
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn settings_update_saves_and_reloads_every_editable_value() {
        let directory = test_directory("save-update");
        let path = directory.join("settings.json");
        let mut settings = Settings::defaults();
        settings.apply_update(SettingsUpdate {
            shell: Some(" /bin/fish ".to_owned()),
            background_image: Some(PathBuf::from("/tmp/wallpaper.png")),
            font_family: "JetBrains Mono".to_owned(),
            font_size: 16.0,
            terminal_padding: TerminalPadding::new(1, 2, 3, 4),
            scrollback_lines: 25_000,
            background_image_opacity: 0.25,
            window_opacity: 0.85,
        });

        write_settings(&path, &settings).unwrap();
        let reloaded = Settings::load_or_create_at(&path).unwrap();

        assert_eq!(reloaded.shell(), Some("/bin/fish"));
        assert_eq!(
            reloaded.background_image(),
            Some(Path::new("/tmp/wallpaper.png"))
        );
        assert_eq!(reloaded.font_family(), "JetBrains Mono");
        assert_eq!(reloaded.font_size(), 16.0);
        assert_eq!(
            reloaded.terminal_padding(),
            TerminalPadding::new(1, 2, 3, 4)
        );
        assert_eq!(reloaded.scrollback_lines(), 25_000);
        assert_eq!(reloaded.background_image_opacity(), 0.25);
        assert_eq!(reloaded.window_opacity(), 0.85);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn invalid_settings_update_values_use_embedded_defaults() {
        let defaults = Settings::defaults();
        let mut settings = defaults.clone();
        settings.apply_update(SettingsUpdate {
            shell: Some("  ".to_owned()),
            background_image: Some(PathBuf::new()),
            font_family: "  ".to_owned(),
            font_size: f64::NAN,
            terminal_padding: TerminalPadding::new(MAX_PADDING + 1, 2, 3, 4),
            scrollback_lines: MAX_SCROLLBACK_LINES + 1,
            background_image_opacity: MAX_BACKGROUND_IMAGE_OPACITY + 0.01,
            window_opacity: MIN_WINDOW_OPACITY - 0.01,
        });

        assert_eq!(settings.shell(), None);
        assert_eq!(settings.background_image(), None);
        assert_eq!(settings.font_family(), defaults.font_family());
        assert_eq!(settings.font_size(), defaults.font_size());
        assert_eq!(
            settings.terminal_padding().top(),
            defaults.terminal_padding().top()
        );
        assert_eq!(settings.terminal_padding().right(), 2);
        assert_eq!(settings.scrollback_lines(), defaults.scrollback_lines());
        assert_eq!(
            settings.background_image_opacity(),
            defaults.background_image_opacity()
        );
        assert_eq!(settings.window_opacity(), defaults.window_opacity());
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
