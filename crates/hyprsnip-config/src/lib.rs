use directories::BaseDirs;
use hyprsnip_utils::TrimOptions;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardSelection {
    Regular,
    Primary,
}

impl Default for ClipboardSelection {
    fn default() -> Self {
        Self::Regular
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    pub poll_interval_ms: u64,
    pub grace_delay_ms: u64,
    pub clipboard: ClipboardSelection,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 250,
            grace_delay_ms: 75,
            clipboard: ClipboardSelection::Regular,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub trim: TrimOptions,
    pub daemon: DaemonConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trim: TrimOptions::default(),
            daemon: DaemonConfig::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("unable to determine config directory")]
    ConfigDirUnavailable,

    #[error("failed reading config file: {path}")]
    ReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed parsing config file: {path}")]
    ParseFailed {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed serializing config")]
    SerializeFailed(#[source] toml::ser::Error),
}

pub fn default_config_path() -> Result<PathBuf, ConfigError> {
    let Some(base) = BaseDirs::new() else {
        return Err(ConfigError::ConfigDirUnavailable);
    };

    Ok(base.config_dir().join("hyprsnip").join("config.toml"))
}

impl Config {
    pub fn load(path_override: Option<&Path>) -> Result<Self, ConfigError> {
        let path = match path_override {
            Some(p) => p.to_path_buf(),
            None => default_config_path()?,
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = std::fs::read_to_string(&path).map_err(|source| ConfigError::ReadFailed {
            path: path.clone(),
            source,
        })?;

        toml::from_str::<Self>(&raw).map_err(|source| ConfigError::ParseFailed { path, source })
    }

    pub fn to_toml_pretty(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(ConfigError::SerializeFailed)
    }

    pub fn default_toml() -> Result<String, ConfigError> {
        Self::default().to_toml_pretty()
    }
}
