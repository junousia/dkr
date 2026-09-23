use std::path::{Path, PathBuf};

use dkr::config::{list_profiles, Profile};
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
    assert_eq!(args, vec!["run", "--rm", "myorg/myimage:latest"]);
}

#[test]
fn interactive_flag_adds_it() {
    let profile = load("minimal");
    let args = build_args(&profile, &[], true);
    assert_eq!(args, vec!["run", "--rm", "-it", "myorg/myimage:latest"]);
}

#[test]
fn trailing_command_is_appended_after_image() {
    let profile = load("minimal");
    let command = vec!["echo".to_string(), "hi".to_string()];
    let args = build_args(&profile, &command, false);
    assert_eq!(
        args,
        vec!["run", "--rm", "myorg/myimage:latest", "echo", "hi"]
    );
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

    let profile = load("expand");
    let args = build_args(&profile, &[], false);

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
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
    assert_eq!(names, vec!["expand", "full", "minimal"]);
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
