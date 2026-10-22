use std::process::{Command, Stdio};

use thiserror::Error;

use crate::config::{Profile, Volume};

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("unterminated '$(' in '{0}'")]
    Unterminated(String),
    #[error("environment variable '{0}' is not set (write $$ for a literal $)")]
    UnsetVar(String),
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

/// Checks, without running anything, that every `$(` in `value` is closed
/// and every `$VAR` outside a command is set. Used by `validate`.
pub fn check_syntax(value: &str) -> Result<(), ResolveError> {
    for segment in split_commands(value)? {
        if let Segment::Literal(text) = segment {
            expand_literal(text, false)?;
        }
    }
    Ok(())
}

fn expand_literal(text: &str, leading: bool) -> Result<String, ResolveError> {
    let expanded = if leading {
        shellexpand::full(text)
    } else {
        shellexpand::env(text)
    };
    expanded
        .map(|s| s.into_owned())
        .map_err(|e| ResolveError::UnsetVar(e.var_name))
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
            // Only the leading literal can start with a `~` to expand.
            Segment::Literal(text) => out.push_str(&expand_literal(text, index == 0)?),
            Segment::Command(command) => out.push_str(&run_command(command)?),
        }
    }
    Ok(out)
}

impl Profile {
    /// Returns a copy with every string field expanded (`~`, `$VAR`,
    /// `$(command)`). Runs commands, so it is not pure. `description` and
    /// `extends` are left alone.
    pub fn resolve(&self) -> Result<Profile, ResolveError> {
        let mut p = self.clone();
        p.image = expand(&p.image)?;
        for volume in &mut p.volumes {
            volume.host = expand(&volume.host)?;
            volume.container = expand(&volume.container)?;
            if let Some(options) = &mut volume.options {
                *options = expand(options)?;
            }
        }
        for value in p.env.values_mut() {
            *value = expand(value)?;
        }
        for field in [
            &mut p.workdir,
            &mut p.user,
            &mut p.network,
            &mut p.entrypoint,
            &mut p.platform,
            &mut p.hostname,
            &mut p.mount_cwd,
        ] {
            if let Some(value) = field {
                *value = expand(value)?;
            }
        }
        for list in [
            &mut p.ports,
            &mut p.extra_args,
            &mut p.env_file,
            &mut p.add_hosts,
        ] {
            for value in list.iter_mut() {
                *value = expand(value)?;
            }
        }
        Ok(p)
    }

    /// Turns `mount_cwd` into a bind mount of the current directory, and
    /// defaults `workdir` to it. Call after `resolve()` and any overrides.
    pub fn apply_mount_cwd(&mut self) -> std::io::Result<()> {
        if let Some(container) = self.mount_cwd.take() {
            let cwd = std::env::current_dir()?;
            if self.workdir.is_none() {
                self.workdir = Some(container.clone());
            }
            self.add_volume(Volume {
                host: cwd.display().to_string(),
                container,
                options: None,
            });
        }
        Ok(())
    }
}
