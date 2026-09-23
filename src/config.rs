use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize, Clone)]
pub struct Volume {
    pub host: String,
    pub container: String,
    #[serde(default)]
    pub options: Option<String>,
}

fn default_init() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone)]
pub struct Profile {
    pub image: String,
    #[serde(default)]
    pub volumes: Vec<Volume>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Run docker's `--init` so the container's main process isn't PID 1,
    /// meaning it gets normal signal handling (e.g. Ctrl-C actually works).
    /// Defaults to true; set to false to opt out.
    #[serde(default = "default_init")]
    pub init: bool,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("profile '{name}' not found in {dir} (available: {available})")]
    NotFound {
        name: String,
        dir: String,
        available: String,
    },
    #[error("failed to read {path}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {path}")]
    Parse {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
}

/// Resolves the profile config directory: an explicit override, else
/// `$DKR_CONFIG_DIR`, else `~/.config/dkr`.
pub fn resolve_config_dir(override_dir: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = override_dir {
        return dir;
    }
    if let Ok(dir) = std::env::var("DKR_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dkr")
}

/// Lists profile names (`.yaml` file stems) available in `config_dir`, sorted.
pub fn list_profiles(config_dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(config_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .filter_map(|entry| {
            entry
                .path()
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .collect();
    names.sort();
    names
}

impl Profile {
    pub fn load(config_dir: &Path, name: &str) -> Result<Profile, ConfigError> {
        let path = config_dir.join(format!("{name}.yaml"));
        let contents = std::fs::read_to_string(&path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                ConfigError::NotFound {
                    name: name.to_string(),
                    dir: config_dir.display().to_string(),
                    available: list_profiles(config_dir).join(", "),
                }
            } else {
                ConfigError::Io {
                    path: path.display().to_string(),
                    source,
                }
            }
        })?;
        serde_yaml::from_str(&contents).map_err(|source| ConfigError::Parse {
            path: path.display().to_string(),
            source,
        })
    }
}
