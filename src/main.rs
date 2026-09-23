use std::process::{Command, ExitCode};

use anyhow::{bail, Context, Result};
use clap::Parser;

use dkr::cli::Cli;
use dkr::config::{list_profiles, resolve_config_dir, schema_json, validate, Profile};
use dkr::docker_cmd;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("dkr: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    if cli.schema {
        println!("{}", schema_json());
        return Ok(ExitCode::SUCCESS);
    }

    let config_dir = resolve_config_dir(cli.config_dir.clone());

    if cli.list {
        let profiles = list_profiles(&config_dir);
        if profiles.is_empty() {
            println!("No profiles found in {}", config_dir.display());
        } else {
            for name in profiles {
                println!("{name}");
            }
        }
        return Ok(ExitCode::SUCCESS);
    }

    let Some(name) = cli.profile.clone() else {
        bail!("a profile name is required (or pass --list to see available profiles)");
    };

    let profile = Profile::load(&config_dir, &name)?;

    if cli.validate {
        let problems = validate(&profile);
        if problems.is_empty() {
            println!("profile '{name}' is valid");
            return Ok(ExitCode::SUCCESS);
        }
        eprintln!("profile '{name}' has problems:");
        for problem in &problems {
            eprintln!("  - {problem}");
        }
        return Ok(ExitCode::FAILURE);
    }

    let interactive = docker_cmd::is_interactive();
    let args = docker_cmd::build_args(&profile, &cli.command, interactive);

    if cli.dry_run {
        println!("docker {}", docker_cmd::shell_join(&args));
        return Ok(ExitCode::SUCCESS);
    }

    let status = Command::new("docker")
        .args(&args)
        .status()
        .context("failed to execute 'docker' - is it installed and on PATH?")?;

    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}
