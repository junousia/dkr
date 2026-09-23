use is_terminal::IsTerminal;

use crate::config::Profile;

/// Whether stdin is an interactive terminal, used to decide whether to pass
/// `-it` to `docker run`.
pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal()
}

fn expand(value: &str) -> String {
    shellexpand::full(value)
        .map(|expanded| expanded.into_owned())
        .unwrap_or_else(|_| value.to_string())
}

/// Builds the full `docker run` argv (everything after `docker`) for the
/// given profile, trailing command, and interactivity. Pure and
/// side-effect-free so it can be tested without touching docker.
pub fn build_args(profile: &Profile, command: &[String], interactive: bool) -> Vec<String> {
    let mut args = vec!["run".to_string(), "--rm".to_string()];
    if interactive {
        args.push("-it".to_string());
    }

    for volume in &profile.volumes {
        let host = expand(&volume.host);
        let mut spec = format!("{host}:{}", volume.container);
        if let Some(options) = &volume.options {
            spec.push(':');
            spec.push_str(options);
        }
        args.push("-v".to_string());
        args.push(spec);
    }

    let mut env_keys: Vec<&String> = profile.env.keys().collect();
    env_keys.sort();
    for key in env_keys {
        let value = expand(&profile.env[key]);
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
