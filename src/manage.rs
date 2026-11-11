use std::process::{Command, ExitCode, Stdio};

use anyhow::{bail, Context, Result};

use crate::docker_cmd::PROFILE_LABEL;

/// argv for `dkr --ps`: every container dkr labelled, running or not.
pub fn ps_args() -> Vec<String> {
    vec![
        "ps".into(),
        "-a".into(),
        format!("--filter=label={PROFILE_LABEL}"),
        "--format".into(),
        format!("table {{{{.Names}}}}\t{{{{.Label \"{PROFILE_LABEL}\"}}}}\t{{{{.Image}}}}\t{{{{.Status}}}}"),
    ]
}

/// argv listing the names of containers started for `profile`.
pub fn find_by_profile_args(profile: &str) -> Vec<String> {
    vec![
        "ps".into(),
        "-a".into(),
        format!("--filter=label={PROFILE_LABEL}={profile}"),
        "--format".into(),
        "{{.Names}}".into(),
    ]
}

/// argv listing containers whose name is exactly `name`.
pub fn find_by_name_args(name: &str) -> Vec<String> {
    vec![
        "ps".into(),
        "-a".into(),
        format!("--filter=name=^/{name}$"),
        "--format".into(),
        "{{.Names}}".into(),
    ]
}

fn docker() -> Command {
    Command::new("docker")
}

fn spawn_error() -> &'static str {
    "failed to execute 'docker' - is it installed and on PATH?"
}

fn output(args: &[String]) -> Result<Vec<String>> {
    let out = docker().args(args).output().context(spawn_error())?;
    if !out.status.success() {
        bail!(
            "docker {} failed: {}",
            args.first().map_or("", String::as_str),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .filter(|line| !line.is_empty())
        .collect())
}

fn passthrough(args: &[String]) -> Result<ExitCode> {
    let status = docker().args(args).status().context(spawn_error())?;
    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}

/// Names of the containers dkr started for `profile`.
pub fn containers_for_profile(profile: &str) -> Result<Vec<String>> {
    output(&find_by_profile_args(profile))
}

pub fn container_exists(name: &str) -> Result<bool> {
    Ok(!output(&find_by_name_args(name))?.is_empty())
}

/// Stops then removes `names`. The removal may find them already gone
/// (containers started with `--rm`), which is fine.
pub fn stop_and_remove(names: &[String]) -> Result<()> {
    let mut stop = vec!["stop".to_string()];
    stop.extend(names.iter().cloned());
    let status = docker().args(&stop).status().context(spawn_error())?;
    if !status.success() {
        bail!("docker stop failed");
    }
    let mut rm = vec!["rm".to_string()];
    rm.extend(names.iter().cloned());
    let _ = docker()
        .args(&rm)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Ok(())
}

pub fn ps() -> Result<ExitCode> {
    passthrough(&ps_args())
}

pub fn stop(profile: &str) -> Result<ExitCode> {
    let names = containers_for_profile(profile)?;
    if names.is_empty() {
        println!("no dkr containers for profile '{profile}'");
        return Ok(ExitCode::SUCCESS);
    }
    stop_and_remove(&names)?;
    Ok(ExitCode::SUCCESS)
}

pub fn logs(profile: &str, follow: bool) -> Result<ExitCode> {
    let names = containers_for_profile(profile)?;
    match names.as_slice() {
        [] => bail!("no dkr containers for profile '{profile}' (see: dkr --ps)"),
        [name] => {
            let mut args = vec!["logs".to_string()];
            if follow {
                args.push("--follow".to_string());
            }
            args.push(name.clone());
            passthrough(&args)
        }
        _ => bail!(
            "profile '{profile}' has several containers ({}); use docker logs <name>",
            names.join(", ")
        ),
    }
}
