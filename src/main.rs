use std::process::{Command, ExitCode};

use anyhow::{bail, Context, Result};
use clap::{CommandFactory, Parser};

use dkr::cli::Cli;
use dkr::config::{
    list_profiles, profile_summaries, resolve_config_dir, schema_json, split_profile_arg, validate,
    with_tag, Profile,
};
use dkr::docker_cmd;
use dkr::manage;

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

    if let Some(shell) = cli.completions {
        clap_complete::generate(shell, &mut Cli::command(), "dkr", &mut std::io::stdout());
        return Ok(ExitCode::SUCCESS);
    }

    let config_dir = resolve_config_dir(cli.config_dir.clone());

    if cli.list && cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&profile_summaries(&config_dir))?
        );
        return Ok(ExitCode::SUCCESS);
    }

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

    if cli.ps {
        return manage::ps();
    }
    if cli.stop || cli.logs {
        let Some(profile) = cli.profile.as_deref() else {
            bail!("--stop and --logs need a profile name");
        };
        let profile = split_profile_arg(profile).0;
        return if cli.stop {
            manage::stop(profile)
        } else {
            manage::logs(profile, cli.follow)
        };
    }

    // In quick mode there is no profile, so the first positional (clap
    // reads it as the profile name) is really the start of the command.
    let mut command = cli.command.clone();
    let (label, mut profile) = match &cli.image {
        Some(image) => {
            command.splice(0..0, cli.profile.clone());
            ("--image".to_string(), Profile::new(image.clone()))
        }
        None => {
            let Some(arg) = cli.profile.clone() else {
                bail!("a profile name or --image is required (or pass --list to see available profiles)");
            };
            let (name, tag_override) = split_profile_arg(&arg);
            let mut profile = Profile::load(&config_dir, name)?;
            if let Some(tag) = tag_override {
                if tag.is_empty() {
                    bail!("empty tag after ':' in '{arg}' - use '<profile>:<tag>'");
                }
                profile.image = with_tag(&profile.image, tag);
            }
            (arg, profile)
        }
    };

    let problems = validate(&profile);
    if cli.validate {
        if problems.is_empty() {
            println!("profile '{label}' is valid (image: {})", profile.image);
            return Ok(ExitCode::SUCCESS);
        }
        eprintln!("profile '{label}' has problems:");
        for problem in &problems {
            eprintln!("  - {problem}");
        }
        return Ok(ExitCode::FAILURE);
    }
    if !problems.is_empty() {
        bail!(
            "profile '{label}' is invalid:\n  - {}",
            problems.join("\n  - ")
        );
    }

    profile = profile.resolve()?;
    cli.apply_overrides(&mut profile)
        .map_err(anyhow::Error::msg)?;
    profile
        .apply_mount_cwd()
        .context("failed to determine the current directory for mount_cwd")?;

    let profile_name = match &cli.image {
        Some(_) => None,
        None => cli.profile.as_deref().map(|arg| split_profile_arg(arg).0),
    };
    docker_cmd::apply_detach_defaults(&mut profile, profile_name).map_err(anyhow::Error::msg)?;

    let build_args = if cli.no_build {
        None
    } else {
        docker_cmd::build_image_args(&profile)
    };
    let interactive = profile.tty.unwrap_or_else(docker_cmd::is_interactive);
    let args = docker_cmd::build_args(&profile, &command, interactive);

    if cli.dry_run {
        if let Some(build_args) = &build_args {
            println!("docker {}", docker_cmd::shell_join(build_args));
        }
        println!("docker {}", docker_cmd::shell_join(&args));
        return Ok(ExitCode::SUCCESS);
    }

    if let Some(build_args) = &build_args {
        let status = Command::new("docker")
            .args(build_args)
            .status()
            .context("failed to execute 'docker' - is it installed and on PATH?")?;
        if !status.success() {
            bail!("docker build failed ({status})");
        }
    }

    if profile.detach {
        let name = profile.name.clone().expect("set by apply_detach_defaults");
        if manage::container_exists(&name)? {
            if !cli.replace {
                bail!(
                    "container '{name}' already exists; pass --replace to recreate it{}",
                    profile_name.map_or(String::new(), |p| format!(" or run: dkr {p} --stop"))
                );
            }
            manage::stop_and_remove(&[name])?;
        }
    }

    let status = Command::new("docker")
        .args(&args)
        .status()
        .context("failed to execute 'docker' - is it installed and on PATH?")?;

    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}
