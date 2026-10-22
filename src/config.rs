use std::collections::HashMap;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use thiserror::Error;

/// A bind mount: `host` is mounted into the container at `container`.
#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(deny_unknown_fields)]
pub struct Volume {
    /// Host path to mount. Supports `~`, `$VAR`/`${VAR}` and `$(command)` expansion.
    pub host: String,
    /// Path inside the container to mount it at.
    pub container: String,
    /// Extra mount options appended after a third `:`, e.g. "ro".
    #[serde(default)]
    pub options: Option<String>,
}

/// When docker pulls the image (`--pull`).
#[derive(Debug, Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lowercase")]
pub enum PullPolicy {
    /// Always pull before running.
    Always,
    /// Pull only if the image is missing locally (docker's default).
    Missing,
    /// Never pull; fail if the image is missing locally.
    Never,
}

impl PullPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            PullPolicy::Always => "always",
            PullPolicy::Missing => "missing",
            PullPolicy::Never => "never",
        }
    }
}

fn default_init() -> bool {
    true
}

/// A named dkr profile: settings applied to `docker run` for one image.
/// Only `image` is required.
#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// One-line human/agent-readable summary of what this profile is for.
    /// Shown by `dkr --list --json`; has no effect on the docker command.
    #[serde(default)]
    pub description: Option<String>,
    /// Name of another profile in the same directory to inherit from.
    /// Scalars set here replace the parent's; `env` is merged (this profile
    /// wins); `volumes`, `ports`, `extra_args`, `env_file` and `add_hosts`
    /// are appended after the parent's (a volume with the same `container`
    /// path replaces the parent's). A parent may itself extend another.
    #[serde(default)]
    pub extends: Option<String>,
    /// The docker image to run, e.g. "myorg/myimage:latest". Supports
    /// `~`, `$VAR`/`${VAR}` and `$(command)` expansion.
    pub image: String,
    /// Mounts the directory dkr is run from at this container path, and
    /// uses it as the working directory unless `workdir` is set.
    #[serde(default)]
    pub mount_cwd: Option<String>,
    /// Files of `KEY=VALUE` lines passed into the container (`--env-file`).
    /// Supports `~`, `$VAR` and `$(command)` expansion.
    #[serde(default)]
    pub env_file: Vec<String>,
    /// When to pull the image (`--pull`): always, missing or never.
    #[serde(default)]
    pub pull: Option<PullPolicy>,
    /// Platform to run the image as, e.g. "linux/amd64" (`--platform`).
    #[serde(default)]
    pub platform: Option<String>,
    /// Container hostname (`--hostname`).
    #[serde(default)]
    pub hostname: Option<String>,
    /// Extra hosts-file entries as "host:ip" strings (`--add-host`).
    #[serde(default)]
    pub add_hosts: Vec<String>,
    /// Force (true) or suppress (false) docker's `-it`. By default it is
    /// used when stdin is a terminal.
    #[serde(default)]
    pub tty: Option<bool>,
    /// Bind mounts (`-v host:container[:options]`).
    #[serde(default)]
    pub volumes: Vec<Volume>,
    /// Environment variables passed into the container (`-e KEY=VALUE`).
    /// Values support `~`, `$VAR`/`${VAR}` and `$(command)` expansion.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Run docker's `--init` so the container's main process isn't PID 1,
    /// meaning it gets normal signal handling (e.g. Ctrl-C actually works).
    /// Defaults to true; set to false to opt out.
    #[serde(default = "default_init")]
    pub init: bool,
    /// Working directory inside the container (`-w`). Supports `$VAR` and
    /// `$(command)` expansion, like every other string field.
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
/// problems; empty means valid. Runs no commands: `$(...)` is only checked
/// for being closed.
pub fn validate(profile: &Profile) -> Vec<String> {
    let mut problems = Vec::new();
    let mut syntax = |field: String, value: &str| {
        if let Err(err) = crate::resolve::check_syntax(value) {
            problems.push(format!("{field}: {err}"));
        }
    };

    syntax("image".into(), &profile.image);
    for (i, volume) in profile.volumes.iter().enumerate() {
        syntax(format!("volumes[{i}].host"), &volume.host);
    }
    for (key, value) in &profile.env {
        syntax(format!("env.{key}"), value);
    }
    for (name, value) in [
        ("workdir", &profile.workdir),
        ("user", &profile.user),
        ("network", &profile.network),
        ("entrypoint", &profile.entrypoint),
        ("platform", &profile.platform),
        ("hostname", &profile.hostname),
        ("mount_cwd", &profile.mount_cwd),
    ] {
        if let Some(value) = value {
            syntax(name.into(), value);
        }
    }
    for (name, list) in [
        ("ports", &profile.ports),
        ("extra_args", &profile.extra_args),
        ("env_file", &profile.env_file),
        ("add_hosts", &profile.add_hosts),
    ] {
        for (i, value) in list.iter().enumerate() {
            syntax(format!("{name}[{i}]"), value);
        }
    }

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
    for key in profile.env.keys() {
        if key.trim().is_empty() {
            problems.push("env has an empty variable name".to_string());
        }
    }
    for (i, port) in profile.ports.iter().enumerate() {
        if !valid_port_spec(port) {
            problems.push(format!(
                "ports[{i}] '{port}' should look like \"[[ip:]host:]container[/proto]\""
            ));
        }
    }
    for (i, arg) in profile.extra_args.iter().enumerate() {
        if arg.trim().is_empty() {
            problems.push(format!("extra_args[{i}] must not be empty"));
        }
    }
    for (i, file) in profile.env_file.iter().enumerate() {
        if file.trim().is_empty() {
            problems.push(format!("env_file[{i}] must not be empty"));
        }
    }
    for (i, host) in profile.add_hosts.iter().enumerate() {
        if !host.contains(':') && !host.contains('$') {
            problems.push(format!(
                "add_hosts[{i}] '{host}' should look like \"host:ip\""
            ));
        }
    }
    if profile
        .mount_cwd
        .as_deref()
        .is_some_and(|p| p.trim().is_empty())
    {
        problems.push("mount_cwd must not be empty".to_string());
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
    #[error("'extends' cycle: {0}")]
    ExtendsCycle(String),
    #[error("{0} must contain a YAML mapping")]
    NotAMapping(String),
    #[error("failed to parse {path}")]
    Parse {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
}

/// Splits a `dkr` positional argument like `myimage:1.0.0` into the profile
/// name to look up and an optional tag override. Profile names never
/// contain `:`, so splitting on the first one is unambiguous.
pub fn split_profile_arg(arg: &str) -> (&str, Option<&str>) {
    match arg.split_once(':') {
        Some((name, tag)) => (name, Some(tag)),
        None => (arg, None),
    }
}

/// Replaces the tag (or digest) on a docker image reference, preserving any
/// registry, which may itself contain a `:port`. Per the image reference
/// grammar, a `:tag` or `@digest` suffix can only appear after the last `/`,
/// so registry ports earlier in the string are never mistaken for it.
pub fn with_tag(image: &str, tag: &str) -> String {
    let tail_start = image.rfind('/').map_or(0, |i| i + 1);
    let cut = image[tail_start..].find(['@', ':']).map(|i| tail_start + i);
    let base = match cut {
        Some(idx) => &image[..idx],
        None => image,
    };
    format!("{base}:{tag}")
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

/// One entry of `dkr --list --json`.
#[derive(Debug, Serialize)]
pub struct ProfileSummary {
    pub name: String,
    pub description: Option<String>,
    pub image: Option<String>,
    /// Set instead of `image`/`description` when the profile fails to parse.
    pub error: Option<String>,
}

/// Summarizes every profile in `config_dir` (sorted by name) for machine
/// consumers; a broken profile is reported in `error` rather than dropped.
pub fn profile_summaries(config_dir: &Path) -> Vec<ProfileSummary> {
    list_profiles(config_dir)
        .into_iter()
        .map(|name| match Profile::load(config_dir, &name) {
            Ok(p) => ProfileSummary {
                name,
                description: p.description,
                image: Some(p.image),
                error: None,
            },
            Err(err) => ProfileSummary {
                name,
                description: None,
                image: None,
                error: Some(format!("{err:#}")),
            },
        })
        .collect()
}

/// Whether `port` is a plausible docker `-p` spec: `[[ip:]host:]container`
/// where `container` is `port[-port][/proto]`. Skipped when it contains `$`
/// (can't know the final value before expansion).
pub fn valid_port_spec(port: &str) -> bool {
    if port.contains('$') {
        return true;
    }
    let container = port.rsplit(':').next().unwrap_or(port);
    let (range, proto) = match container.split_once('/') {
        Some((range, proto)) => (range, Some(proto)),
        None => (container, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let range_ok = match range.split_once('-') {
        Some((a, b)) => digits(a) && digits(b),
        None => digits(range),
    };
    range_ok && proto.is_none_or(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphabetic()))
}

fn read_profile_file(config_dir: &Path, name: &str) -> Result<(PathBuf, String), ConfigError> {
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
    Ok((path, contents))
}

fn parse_error(path: &Path, source: serde_yaml::Error) -> ConfigError {
    ConfigError::Parse {
        path: path.display().to_string(),
        source,
    }
}

const APPENDED_LISTS: [&str; 5] = ["volumes", "ports", "extra_args", "env_file", "add_hosts"];

fn container_of(volume: &Value) -> Option<&str> {
    volume.get("container").and_then(Value::as_str)
}

/// Overlays `child` onto `base` per the `extends` rules documented on
/// [`Profile::extends`].
fn merge_profiles(base: Value, child: Value) -> Value {
    let (Value::Mapping(mut merged), Value::Mapping(child)) = (base, child) else {
        unreachable!("profiles are checked to be mappings before merging");
    };
    for (key, value) in child {
        let name = key.as_str().unwrap_or_default();
        let combined = match (merged.remove(&key), value) {
            (Some(Value::Mapping(mut b)), Value::Mapping(c)) if name == "env" => {
                for (k, v) in c {
                    b.insert(k, v);
                }
                Value::Mapping(b)
            }
            (Some(Value::Sequence(mut b)), Value::Sequence(c))
                if APPENDED_LISTS.contains(&name) =>
            {
                if name == "volumes" {
                    b.retain(|bv| {
                        container_of(bv)
                            .is_none_or(|bc| !c.iter().any(|cv| container_of(cv) == Some(bc)))
                    });
                }
                b.extend(c);
                Value::Sequence(b)
            }
            (_, value) => value,
        };
        merged.insert(key, combined);
    }
    Value::Mapping(merged)
}

fn load_merged(
    config_dir: &Path,
    name: &str,
    chain: &mut Vec<String>,
) -> Result<Value, ConfigError> {
    if chain.iter().any(|seen| seen == name) {
        chain.push(name.to_string());
        return Err(ConfigError::ExtendsCycle(chain.join(" -> ")));
    }
    chain.push(name.to_string());
    let (path, contents) = read_profile_file(config_dir, name)?;
    let value: Value = serde_yaml::from_str(&contents).map_err(|e| parse_error(&path, e))?;
    if !value.is_mapping() {
        return Err(ConfigError::NotAMapping(path.display().to_string()));
    }
    match value.get("extends").and_then(Value::as_str) {
        Some(parent) => {
            let base = load_merged(config_dir, parent, chain)?;
            Ok(merge_profiles(base, value))
        }
        None => Ok(value),
    }
}

impl Profile {
    /// A profile that runs `image` with every setting at its default.
    pub fn new(image: impl Into<String>) -> Profile {
        Profile {
            description: None,
            extends: None,
            image: image.into(),
            mount_cwd: None,
            env_file: Vec::new(),
            pull: None,
            platform: None,
            hostname: None,
            add_hosts: Vec::new(),
            tty: None,
            volumes: Vec::new(),
            env: HashMap::new(),
            init: true,
            workdir: None,
            user: None,
            network: None,
            ports: Vec::new(),
            entrypoint: None,
            extra_args: Vec::new(),
        }
    }

    /// Adds a bind mount, replacing any existing one at the same container
    /// path (docker rejects duplicate mount points).
    pub fn add_volume(&mut self, volume: Volume) {
        self.volumes.retain(|v| v.container != volume.container);
        self.volumes.push(volume);
    }

    pub fn load(config_dir: &Path, name: &str) -> Result<Profile, ConfigError> {
        let (path, contents) = read_profile_file(config_dir, name)?;
        let value: Value = serde_yaml::from_str(&contents).map_err(|e| parse_error(&path, e))?;
        if value.get("extends").is_some() {
            let merged = load_merged(config_dir, name, &mut Vec::new())?;
            return serde_yaml::from_value(merged).map_err(|e| parse_error(&path, e));
        }
        // No inheritance: parse the text directly so errors keep line numbers.
        serde_yaml::from_str(&contents).map_err(|e| parse_error(&path, e))
    }
}
