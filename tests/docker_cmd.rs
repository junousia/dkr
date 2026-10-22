use std::path::{Path, PathBuf};

use dkr::config::{list_profiles, validate, with_tag, Profile};
use dkr::docker_cmd::{build_args, shell_join};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load(name: &str) -> Profile {
    Profile::load(&fixtures_dir(), name).expect("fixture should parse")
}

#[test]
fn minimal_profile_builds_bare_run_command() {
    let profile = load("minimal");
    let args = build_args(&profile, &[], false);
    assert_eq!(args, vec!["run", "--rm", "--init", "myorg/myimage:latest"]);
}

#[test]
fn tag_override_end_to_end_replaces_the_image_tag() {
    let mut profile = load("minimal");
    profile.image = with_tag(&profile.image, "1.0.0");
    let args = build_args(&profile, &[], false);
    assert_eq!(args, vec!["run", "--rm", "--init", "myorg/myimage:1.0.0"]);
}

#[test]
fn interactive_flag_adds_it() {
    let profile = load("minimal");
    let args = build_args(&profile, &[], true);
    assert_eq!(
        args,
        vec!["run", "--rm", "--init", "-it", "myorg/myimage:latest"]
    );
}

#[test]
fn trailing_command_is_appended_after_image() {
    let profile = load("minimal");
    let command = vec!["echo".to_string(), "hi".to_string()];
    let args = build_args(&profile, &command, false);
    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "--init",
            "myorg/myimage:latest",
            "echo",
            "hi"
        ]
    );
}

#[test]
fn init_false_omits_the_init_flag() {
    let profile = load("no_init");
    let args = build_args(&profile, &[], false);
    assert_eq!(args, vec!["run", "--rm", "myorg/myimage:latest"]);
}

#[test]
fn full_profile_orders_every_flag_correctly() {
    let profile = load("full");
    let args = build_args(&profile, &[], false);
    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "--init",
            "-v",
            "/home/jukka/project:/workspace",
            "-v",
            "/home/jukka/cache:/cache:ro",
            "-e",
            "BAZ=qux",
            "-e",
            "FOO=bar",
            "-w",
            "/workspace",
            "-u",
            "1000:1000",
            "--network",
            "host",
            "-p",
            "8080:80",
            "--entrypoint",
            "/bin/bash",
            "--cap-add=SYS_PTRACE",
            "myorg/myimage:latest",
        ]
    );
}

#[test]
fn env_and_volume_values_expand_tilde_and_vars() {
    unsafe {
        std::env::set_var("DKR_TEST_HOME", "/home/jukka");
        std::env::set_var("DKR_TEST_NAME", "jukka");
    }

    let profile = load("expand").resolve().unwrap();
    let args = build_args(&profile, &[], false);

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "--init",
            "-v",
            "/home/jukka/project:/workspace",
            "-e",
            "GREETING=hello jukka",
            "myorg/myimage:latest",
        ]
    );
}

#[test]
fn list_profiles_finds_fixture_names_sorted() {
    let names = list_profiles(&fixtures_dir());
    assert_eq!(
        names,
        vec![
            "base",
            "child",
            "command_subst",
            "expand",
            "extras",
            "full",
            "minimal",
            "no_init"
        ]
    );
}

#[test]
fn missing_profile_lists_available_names_in_error() {
    let err = Profile::load(&fixtures_dir(), "does-not-exist").unwrap_err();
    let message = err.to_string();
    assert!(message.contains("does-not-exist"));
    assert!(message.contains("expand"));
    assert!(message.contains("full"));
    assert!(message.contains("minimal"));
}

#[test]
fn shell_join_quotes_arguments_with_special_characters() {
    let args = vec![
        "run".to_string(),
        "hello world".to_string(),
        "plain".to_string(),
    ];
    assert_eq!(shell_join(&args), "run 'hello world' plain");
}

#[test]
fn command_substitution_runs_in_env_and_volume_values() {
    let profile = load("command_subst").resolve().unwrap();
    let args = build_args(&profile, &[], false);
    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "--init",
            "-v",
            "/data/abc:/workspace",
            "-e",
            "LITERAL=$(not run)",
            "-e",
            "NESTED=outer-inner",
            "-e",
            "TOKEN=s3cr$t ~ ${x}",
            "myorg/myimage:latest",
        ]
    );
}

#[test]
fn failing_command_substitution_is_an_error() {
    let mut profile = load("minimal");
    profile.env.insert("X".into(), "$(exit 3)".into());
    let err = profile.resolve().unwrap_err().to_string();
    assert!(err.contains("exit 3"), "{err}");
}

#[test]
fn validate_flags_unterminated_substitution_without_running_it() {
    let mut profile = load("minimal");
    profile.env.insert("X".into(), "$(echo hi".into());
    let problems = validate(&profile);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("env.X"));
}

#[test]
fn new_first_class_fields_map_to_docker_flags() {
    let args = build_args(&load("extras"), &[], false);
    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "--init",
            "--pull",
            "never",
            "--platform",
            "linux/amd64",
            "--hostname",
            "box",
            "--add-host",
            "db:10.0.0.5",
            "--env-file",
            "/etc/app.env",
            "-e",
            "X=1",
            "myorg/myimage:latest",
        ]
    );
}

#[test]
fn extends_merges_parent_and_child() {
    let profile = load("child");
    assert_eq!(profile.image, "myorg/child:2");
    assert_eq!(profile.description.as_deref(), Some("Child of base"));
    assert_eq!(profile.user.as_deref(), Some("1000:1000"));
    let args = build_args(&profile, &[], false);
    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "--init",
            "-v",
            "/cache:/cache",
            "-v",
            "/new:/workspace",
            "-e",
            "A=base",
            "-e",
            "B=child",
            "-e",
            "C=child",
            "-u",
            "1000:1000",
            "-p",
            "80:80",
            "-p",
            "81:81",
            "--base",
            "--child",
            "myorg/child:2",
        ]
    );
}

#[test]
fn extends_cycle_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.yaml"), "extends: b\nimage: x\n").unwrap();
    std::fs::write(dir.path().join("b.yaml"), "extends: a\nimage: x\n").unwrap();
    let err = Profile::load(dir.path(), "a").unwrap_err().to_string();
    assert!(err.contains("a -> b -> a"), "{err}");
}

#[test]
fn extends_missing_parent_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.yaml"), "extends: nope\nimage: x\n").unwrap();
    let err = Profile::load(dir.path(), "a").unwrap_err().to_string();
    assert!(err.contains("nope"), "{err}");
}

#[test]
fn command_substitution_covers_extra_args_and_other_fields() {
    let mut profile = load("minimal");
    profile.extra_args.push("--label=host=$(echo box)".into());
    profile.user = Some("$(echo 7):7".into());
    let args = build_args(&profile.resolve().unwrap(), &[], false);
    assert!(args.contains(&"--label=host=box".to_string()));
    assert!(args.contains(&"7:7".to_string()));
}

#[test]
fn mount_cwd_mounts_current_dir_and_defaults_workdir() {
    let mut profile = load("minimal");
    profile.mount_cwd = Some("/src".into());
    profile.apply_mount_cwd().unwrap();
    let cwd = std::env::current_dir().unwrap().display().to_string();
    let args = build_args(&profile, &[], false);
    assert_eq!(
        args,
        vec![
            "run".to_string(),
            "--rm".into(),
            "--init".into(),
            "-v".into(),
            format!("{cwd}:/src"),
            "-w".into(),
            "/src".into(),
            "myorg/myimage:latest".into(),
        ]
    );
}

#[test]
fn cli_overrides_apply_on_top_of_a_profile() {
    use clap::Parser;
    let cli = dkr::cli::Cli::parse_from([
        "dkr",
        "full",
        "-e",
        "FOO=cli",
        "-e",
        "NEW=1",
        "-v",
        "/x:/cache:rw",
        "-p",
        "9000",
        "-w",
        "/tmp",
        "--pull",
        "always",
        "-a",
        "--privileged",
        "--",
        "ls",
    ]);
    assert_eq!(cli.command, vec!["ls"]);
    let mut profile = load("full");
    cli.apply_overrides(&mut profile).unwrap();
    let args = build_args(&profile, &cli.command, false);
    assert!(args.windows(2).any(|w| w == ["-e", "FOO=cli"]));
    assert!(args.windows(2).any(|w| w == ["-e", "NEW=1"]));
    assert!(args.windows(2).any(|w| w == ["-v", "/x:/cache:rw"]));
    assert!(!args.iter().any(|a| a == "/home/jukka/cache:/cache:ro"));
    assert!(args.windows(2).any(|w| w == ["-p", "9000"]));
    assert!(args.windows(2).any(|w| w == ["-w", "/tmp"]));
    assert!(args.windows(2).any(|w| w == ["--pull", "always"]));
    assert_eq!(args.iter().filter(|a| *a == "--privileged").count(), 1);
}

#[test]
fn cli_rejects_bad_override_values() {
    use clap::Parser;
    let mut profile = load("minimal");
    for bad in [["-p", "nope"], ["-v", "justone"], ["-e", "=x"]] {
        let cli = dkr::cli::Cli::parse_from(["dkr", "minimal", bad[0], bad[1]]);
        assert!(cli.apply_overrides(&mut profile).is_err(), "{bad:?}");
    }
}
