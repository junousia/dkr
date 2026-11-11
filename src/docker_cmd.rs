use is_terminal::IsTerminal;

use crate::config::Profile;

/// Whether stdin is an interactive terminal, used to decide whether to pass
/// `-it` to `docker run`.
pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal()
}

/// Label put on every detached container so `--ps`, `--stop` and `--logs`
/// can find the containers dkr started, whatever they are named.
pub const PROFILE_LABEL: &str = "dkr.profile";

/// Fills in what a detached profile needs: a container name (default
/// `dkr-<profile_name>`) and the [`PROFILE_LABEL`] label. No-op for
/// foreground profiles. Errors if a name can't be determined.
pub fn apply_detach_defaults(
    profile: &mut Profile,
    profile_name: Option<&str>,
) -> Result<(), String> {
    if !profile.detach {
        return Ok(());
    }
    if profile.name.is_none() {
        let Some(profile_name) = profile_name else {
            return Err("a detached --image run needs --name".to_string());
        };
        profile.name = Some(format!("dkr-{profile_name}"));
    }
    let label_value = profile_name
        .map(str::to_string)
        .or_else(|| profile.name.clone())
        .expect("name or profile_name is set");
    profile
        .extra_args
        .push(format!("--label={PROFILE_LABEL}={label_value}"));
    Ok(())
}

/// Builds the `docker build` argv (everything after `docker`) for a profile
/// with a `build:` section, or `None` if it has none.
pub fn build_image_args(profile: &Profile) -> Option<Vec<String>> {
    let build = profile.build.as_ref()?;
    let mut args = vec!["build".to_string(), "-t".to_string(), profile.image.clone()];
    if let Some(dockerfile) = &build.dockerfile {
        args.push("-f".to_string());
        args.push(dockerfile.clone());
    }
    if let Some(target) = &build.target {
        args.push("--target".to_string());
        args.push(target.clone());
    }
    if let Some(platform) = &profile.platform {
        args.push("--platform".to_string());
        args.push(platform.clone());
    }
    let mut keys: Vec<&String> = build.args.keys().collect();
    keys.sort();
    for key in keys {
        args.push("--build-arg".to_string());
        args.push(format!("{key}={}", build.args[key]));
    }
    args.push(build.context.clone().unwrap_or_else(|| ".".to_string()));
    Some(args)
}

/// Builds the full `docker run` argv (everything after `docker`) for the
/// given profile, trailing command, and interactivity. Pure and
/// side-effect-free so it can be tested without touching docker. Expects
/// `~`/`$VAR`/`$(command)` to be resolved already (see `Profile::resolve`).
pub fn build_args(profile: &Profile, command: &[String], interactive: bool) -> Vec<String> {
    let mut args = vec!["run".to_string()];
    if profile.detach {
        args.push("-d".to_string());
    }
    // docker rejects --rm together with a restart policy.
    if profile.restart.is_none() {
        args.push("--rm".to_string());
    }
    if let Some(name) = &profile.name {
        args.push("--name".to_string());
        args.push(name.clone());
    }
    if let Some(restart) = &profile.restart {
        args.push("--restart".to_string());
        args.push(restart.clone());
    }
    if profile.init {
        args.push("--init".to_string());
    }
    if interactive && !profile.detach {
        args.push("-it".to_string());
    }

    if let Some(pull) = profile.pull {
        args.push("--pull".to_string());
        args.push(pull.as_str().to_string());
    }
    if let Some(platform) = &profile.platform {
        args.push("--platform".to_string());
        args.push(platform.clone());
    }
    if let Some(hostname) = &profile.hostname {
        args.push("--hostname".to_string());
        args.push(hostname.clone());
    }
    for host in &profile.add_hosts {
        args.push("--add-host".to_string());
        args.push(host.clone());
    }

    for volume in &profile.volumes {
        let host = &volume.host;
        let mut spec = format!("{host}:{}", volume.container);
        if let Some(options) = &volume.options {
            spec.push(':');
            spec.push_str(options);
        }
        args.push("-v".to_string());
        args.push(spec);
    }

    for file in &profile.env_file {
        args.push("--env-file".to_string());
        args.push(file.clone());
    }

    let mut env_keys: Vec<&String> = profile.env.keys().collect();
    env_keys.sort();
    for key in env_keys {
        let value = &profile.env[key];
        args.push("-e".to_string());
        args.push(format!("{key}={value}"));
    }

    if let Some(workdir) = &profile.workdir {
        args.push("-w".to_string());
        args.push(workdir.clone());
    }
    if let Some(user) = &profile.user {
        args.push("-u".to_string());
        args.push(user.clone());
    }
    if let Some(network) = &profile.network {
        args.push("--network".to_string());
        args.push(network.clone());
    }
    for port in &profile.ports {
        args.push("-p".to_string());
        args.push(port.clone());
    }
    if let Some(entrypoint) = &profile.entrypoint {
        args.push("--entrypoint".to_string());
        args.push(entrypoint.clone());
    }

    args.extend(profile.extra_args.iter().cloned());
    args.push(profile.image.clone());
    args.extend(command.iter().cloned());
    args
}

/// Quotes an argv for human-readable `--dry-run` output.
pub fn shell_join(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            let needs_quoting = arg.is_empty()
                || arg
                    .chars()
                    .any(|c| c.is_whitespace() || "\"'$`\\".contains(c));
            if needs_quoting {
                format!("'{}'", arg.replace('\'', "'\\''"))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
