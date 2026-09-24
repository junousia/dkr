use std::path::PathBuf;

use clap::Parser;

/// Run one-off commands from docker images using named per-image profiles.
#[derive(Parser, Debug)]
#[command(name = "dkr", version, about)]
pub struct Cli {
    /// List available profiles and exit.
    #[arg(short, long)]
    pub list: bool,

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
