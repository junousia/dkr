use std::path::PathBuf;

use clap::Parser;
use clap_complete::Shell;

use crate::config::{valid_port_spec, Profile, PullPolicy, Volume};

/// Run one-off commands from docker images using named per-image profiles.
#[derive(Parser, Debug)]
#[command(
    name = "dkr",
    version,
    about,
    after_help = "For scripts and AI agents: `dkr --list --json` shows profiles, `dkr --schema` \
prints the profile format, `dkr <profile> --validate` checks one, and `dkr <profile> --dry-run` \
shows the docker command without running it."
)]
pub struct Cli {
    /// List available profiles and exit.
    #[arg(short, long)]
    pub list: bool,

    /// With --list, print a JSON array of {name, description, image, error}
    /// instead of bare names (for scripts and agents).
    #[arg(long, requires = "list")]
    pub json: bool,

    /// Print the docker command that would run, without executing it.
    #[arg(long)]
    pub dry_run: bool,

    /// Print the profile JSON Schema (for editor autocomplete/validation or
    /// standalone tooling) and exit. Ignores any profile argument.
    #[arg(long)]
    pub schema: bool,

    /// Validate the given profile (structure and field sanity) without
    /// running docker.
    #[arg(long)]
    pub validate: bool,

    /// Quick mode: run this image with no profile file. Any positional
    /// arguments then form the command, e.g. `dkr --image alpine -- echo hi`.
    #[arg(long, value_name = "IMAGE")]
    pub image: Option<String>,

    /// Add/override an env var for this run: KEY=VALUE, or KEY to pass the
    /// host's value through. Repeatable.
    #[arg(short = 'e', long = "env", value_name = "KEY[=VALUE]")]
    pub env: Vec<String>,

    /// Add a bind mount for this run: HOST:CONTAINER[:OPTIONS]. Replaces a
    /// profile mount at the same container path. Repeatable.
    #[arg(short = 'v', long = "volume", value_name = "HOST:CONTAINER[:OPTS]")]
    pub volume: Vec<String>,

    /// Publish a port for this run: [[IP:]HOST:]CONTAINER[/PROTO]. Repeatable.
    #[arg(short = 'p', long = "publish", value_name = "PORTSPEC")]
    pub publish: Vec<String>,

    /// Mount the current directory at this container path (and use it as
    /// the working directory unless -w is given).
    #[arg(long, value_name = "CONTAINER_PATH")]
    pub mount_cwd: Option<String>,

    /// Override the working directory inside the container.
    #[arg(short = 'w', long)]
    pub workdir: Option<String>,

    /// Override the user to run as inside the container.
    #[arg(short = 'u', long)]
    pub user: Option<String>,

    /// Override the docker network mode.
    #[arg(long)]
    pub network: Option<String>,

    /// Override the image's entrypoint.
    #[arg(long)]
    pub entrypoint: Option<String>,

    /// Override the platform, e.g. linux/amd64.
    #[arg(long)]
    pub platform: Option<String>,

    /// Override when the image is pulled.
    #[arg(long, value_enum)]
    pub pull: Option<PullPolicy>,

    /// Pass one raw extra `docker run` flag for this run, e.g.
    /// `-a --cap-add=SYS_PTRACE`. Repeatable; appended after the profile's
    /// extra_args.
    #[arg(
        short = 'a',
        long = "arg",
        allow_hyphen_values = true,
        value_name = "FLAG"
    )]
    pub arg: Vec<String>,

    /// Print a shell completion script (bash, zsh, fish, elvish,
    /// powershell) and exit. Ignores any profile argument.
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<Shell>,

    /// Directory containing profile YAML files (overrides $DKR_CONFIG_DIR and ~/.config/dkr).
    #[arg(long)]
    pub config_dir: Option<PathBuf>,

    /// Profile name, matching <config-dir>/<profile>.yaml. Required unless
    /// --list. Append ":tag" (e.g. "myimage:1.0.0") to override the tag on
    /// the profile's image for this run only.
    pub profile: Option<String>,

    /// Command and arguments to run inside the container, overriding the image's default CMD.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

impl Cli {
    /// Applies this run's flag overrides to an already-resolved `profile`.
    /// Values are used verbatim (the shell has expanded them already), so
    /// this must run after `Profile::resolve`.
    pub fn apply_overrides(&self, profile: &mut Profile) -> Result<(), String> {
        for entry in &self.env {
            match entry.split_once('=') {
                Some(("", _)) | None if entry.is_empty() || entry.starts_with('=') => {
                    return Err(format!(
                        "invalid --env '{entry}': expected KEY or KEY=VALUE"
                    ));
                }
                Some((key, value)) => {
                    profile.env.insert(key.to_string(), value.to_string());
                }
                None => {
                    if let Ok(value) = std::env::var(entry) {
                        profile.env.insert(entry.clone(), value);
                    }
                }
            }
        }
        for spec in &self.volume {
            let mut parts = spec.splitn(3, ':');
            let (host, container, options) = (parts.next(), parts.next(), parts.next());
            match (host, container) {
                (Some(h), Some(c)) if !h.is_empty() && !c.is_empty() => {
                    profile.add_volume(Volume {
                        host: h.to_string(),
                        container: c.to_string(),
                        options: options.map(str::to_string),
                    });
                }
                _ => {
                    return Err(format!(
                        "invalid --volume '{spec}': expected HOST:CONTAINER[:OPTIONS]"
                    ))
                }
            }
        }
        for port in &self.publish {
            if !valid_port_spec(port) {
                return Err(format!(
                    "invalid --publish '{port}': expected [[IP:]HOST:]CONTAINER[/PROTO]"
                ));
            }
            profile.ports.push(port.clone());
        }
        if let Some(path) = &self.mount_cwd {
            profile.mount_cwd = Some(path.clone());
        }
        if let Some(v) = &self.workdir {
            profile.workdir = Some(v.clone());
        }
        if let Some(v) = &self.user {
            profile.user = Some(v.clone());
        }
        if let Some(v) = &self.network {
            profile.network = Some(v.clone());
        }
        if let Some(v) = &self.entrypoint {
            profile.entrypoint = Some(v.clone());
        }
        if let Some(v) = &self.platform {
            profile.platform = Some(v.clone());
        }
        if let Some(v) = self.pull {
            profile.pull = Some(v);
        }
        profile.extra_args.extend(self.arg.iter().cloned());
        Ok(())
    }
}
