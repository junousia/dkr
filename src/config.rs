use std::collections::HashMap;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;
use thiserror::Error;

/// A bind mount: `host` is mounted into the container at `container`.
#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(deny_unknown_fields)]
pub struct Volume {
    /// Host path to mount. Supports `~` and `$VAR`/`${VAR}` expansion.
    pub host: String,
    /// Path inside the container to mount it at.
    pub container: String,
    /// Extra mount options appended after a third `:`, e.g. "ro".
    #[serde(default)]
    pub options: Option<String>,
}

fn default_init() -> bool {
    true
}

/// A named dkr profile: settings applied to `docker run` for one image.
/// Only `image` is required.
#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// The docker image to run, e.g. "myorg/myimage:latest".
    pub image: String,
    /// Bind mounts (`-v host:container[:options]`).
    #[serde(default)]
    pub volumes: Vec<Volume>,
    /// Environment variables passed into the container (`-e KEY=VALUE`).
    /// Values support `~` and `$VAR`/`${VAR}` expansion.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Run docker's `--init` so the container's main process isn't PID 1,
    /// meaning it gets normal signal handling (e.g. Ctrl-C actually works).
    /// Defaults to true; set to false to opt out.
    #[serde(default = "default_init")]
    pub init: bool,
    /// Working directory inside the container (`-w`).
    #[serde(default)]
    pub workdir: Option<String>,
    /// User to run as inside the container, e.g. "1000:1000" (`-u`).
    #[serde(default)]
    pub user: Option<String>,
    /// Docker network mode, e.g. "host" (`--network`).
    #[serde(default)]
    pub network: Option<String>,
    /// Port mappings as "host:container" strings (`-p`).
    #[serde(default)]
    pub ports: Vec<String>,
    /// Overrides the image's entrypoint (`--entrypoint`).
    #[serde(default)]
    pub entrypoint: Option<String>,
    /// Raw extra `docker run` flags, appended verbatim before the image.
    #[serde(default)]
    pub extra_args: Vec<String>,
}

/// Returns the JSON Schema for [`Profile`], pretty-printed. Derived directly
/// from the struct, so it can never drift from the fields dkr actually reads.
pub fn schema_json() -> String {
    let schema = schemars::schema_for!(Profile);
    serde_json::to_string_pretty(&schema).expect("schema serializes to JSON")
}

/// Validates a profile beyond what YAML parsing already checks (structural
/// validity, e.g. rejecting unknown fields). Returns a list of human-readable
/// problems; empty means valid.
pub fn validate(profile: &Profile) -> Vec<String> {
    let mut problems = Vec::new();

    if profile.image.trim().is_empty() {
        problems.push("image must not be empty".to_string());
    }
    for (i, volume) in profile.volumes.iter().enumerate() {
        if volume.host.trim().is_empty() {
            problems.push(format!("volumes[{i}].host must not be empty"));
        }
        if volume.container.trim().is_empty() {
            problems.push(format!("volumes[{i}].container must not be empty"));
        }
    }
    for (key, _) in &profile.env {
        if key.trim().is_empty() {
            problems.push("env has an empty variable name".to_string());
        }
    }
    for (i, port) in profile.ports.iter().enumerate() {
        if !port.contains(':') {
            problems.push(format!(
                "ports[{i}] '{port}' should look like \"host:container\""
            ));
        }
    }
    for (i, arg) in profile.extra_args.iter().enumerate() {
        if arg.trim().is_empty() {
            problems.push(format!("extra_args[{i}] must not be empty"));
        }
    }

    problems
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
