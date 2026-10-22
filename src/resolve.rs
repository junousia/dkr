use std::process::{Command, Stdio};

use thiserror::Error;

use crate::config::Profile;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("unterminated '$(' in '{0}'")]
    Unterminated(String),
    #[error("failed to run '{command}'")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("command '{command}' exited with {status}")]
    Failed { command: String, status: String },
    #[error("command '{0}' produced non-UTF-8 output")]
    NotUtf8(String),
}

enum Segment<'a> {
    Literal(&'a str),
    Command(&'a str),
}

/// Splits `value` into literal text and `$(...)` command bodies. Parentheses
/// nest; `$$` is left in the literal so `$$(x)` stays an escaped `$(x)` once
/// shell expansion turns `$$` into `$`.
fn split_commands(value: &str) -> Result<Vec<Segment<'_>>, ResolveError> {
    let bytes = value.as_bytes();
    let mut segments = Vec::new();
    let (mut literal_start, mut i) = (0, 0);
    while i < bytes.len() {
        if bytes[i] == b'$' && bytes.get(i + 1) == Some(&b'$') {
            i += 2;
        } else if bytes[i] == b'$' && bytes.get(i + 1) == Some(&b'(') {
            let body_start = i + 2;
            let mut depth = 1;
            let mut j = body_start;
            while j < bytes.len() && depth > 0 {
                match bytes[j] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            if depth > 0 {
                return Err(ResolveError::Unterminated(value.to_string()));
            }
            if literal_start < i {
                segments.push(Segment::Literal(&value[literal_start..i]));
            }
            segments.push(Segment::Command(&value[body_start..j - 1]));
            literal_start = j;
            i = j;
        } else {
            i += 1;
        }
    }
    if literal_start < bytes.len() {
        segments.push(Segment::Literal(&value[literal_start..]));
    }
    Ok(segments)
}

/// Checks that every `$(` in `value` is closed, without running anything.
/// Used by `--validate`.
pub fn check_syntax(value: &str) -> Result<(), ResolveError> {
    split_commands(value).map(|_| ())
}

fn run_command(command: &str) -> Result<String, ResolveError> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|source| ResolveError::Spawn {
            command: command.to_string(),
            source,
        })?;
    if !output.status.success() {
        return Err(ResolveError::Failed {
            command: command.to_string(),
            status: output.status.to_string(),
        });
    }
    let text =
        String::from_utf8(output.stdout).map_err(|_| ResolveError::NotUtf8(command.to_string()))?;
    Ok(text.trim_end_matches('\n').to_string())
}

/// Expands `~`, `$VAR`/`${VAR}` and `$(command)` in `value`. Command output
/// is inserted verbatim, never re-expanded.
pub fn expand(value: &str) -> Result<String, ResolveError> {
    let mut out = String::new();
    for (index, segment) in split_commands(value)?.into_iter().enumerate() {
        match segment {
            Segment::Literal(text) => {
                // Only the leading literal can start with a `~` to expand.
                let expanded = if index == 0 {
                    shellexpand::full(text).map(|s| s.into_owned())
                } else {
                    shellexpand::env(text).map(|s| s.into_owned())
                };
                out.push_str(&expanded.unwrap_or_else(|_| text.to_string()));
            }
            Segment::Command(command) => out.push_str(&run_command(command)?),
        }
    }
    Ok(out)
}

impl Profile {
    /// Returns a copy with `volumes[].host` and `env` values expanded
    /// (`~`, `$VAR`, `$(command)`). Runs commands, so it is not pure.
    pub fn resolve(&self) -> Result<Profile, ResolveError> {
        let mut resolved = self.clone();
        for volume in &mut resolved.volumes {
            volume.host = expand(&volume.host)?;
        }
        for value in resolved.env.values_mut() {
            *value = expand(value)?;
        }
        Ok(resolved)
    }
}
