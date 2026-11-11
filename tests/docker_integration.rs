//! End-to-end tests against a real docker daemon. Ignored by default; run
//! with `make test-docker` (needs `docker` on PATH and network for alpine).

use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn dkr(config_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dkr"))
        .arg("--config-dir")
        .arg(config_dir)
        .args(args)
        .output()
        .expect("run dkr")
}

fn docker(args: &[&str]) -> Output {
    Command::new("docker")
        .args(args)
        .output()
        .expect("run docker")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn unique(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if done() {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("timed out waiting for {what}");
}

fn exists(name: &str) -> bool {
    !stdout(&docker(&[
        "ps",
        "-a",
        "-q",
        "--filter",
        &format!("name=^/{name}$"),
    ]))
    .is_empty()
}

#[test]
#[ignore = "needs docker"]
fn quick_mode_runs_a_command_in_a_container() {
    let dir = tempfile::tempdir().unwrap();
    let out = dkr(
        dir.path(),
        &[
            "--image",
            "alpine:3",
            "-e",
            "WHO=dkr",
            "--",
            "sh",
            "-c",
            "echo hi $WHO",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(stdout(&out), "hi dkr");
}

#[test]
#[ignore = "needs docker"]
fn build_profile_builds_then_runs() {
    let dir = tempfile::tempdir().unwrap();
    let tag = unique("dkr-it-build");
    std::fs::write(
        dir.path().join("Dockerfile"),
        "FROM alpine:3\nARG GREETING=none\nRUN echo $GREETING > /greeting\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("app.yaml"),
        format!(
            "image: {tag}:latest\nbuild:\n  context: {ctx}\n  args:\n    GREETING: \"$(echo built-ok)\"\n",
            ctx = dir.path().display()
        ),
    )
    .unwrap();

    let out = dkr(dir.path(), &["app", "--", "cat", "/greeting"]);
    let _ = docker(&["rmi", "-f", &format!("{tag}:latest")]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(stdout(&out).lines().last().unwrap(), "built-ok");
}

#[test]
#[ignore = "needs docker"]
fn detached_profile_lifecycle_ps_logs_replace_stop() {
    let dir = tempfile::tempdir().unwrap();
    let profile = unique("svc");
    std::fs::write(
        dir.path().join(format!("{profile}.yaml")),
        "image: alpine:3\ndetach: true\n",
    )
    .unwrap();
    let name = format!("dkr-{profile}");
    let cmd = ["sh", "-c", "echo started; sleep 300"];
    let run = |extra: &[&str]| {
        let mut args = vec![profile.as_str()];
        args.extend(extra);
        args.push("--");
        args.extend(cmd);
        dkr(dir.path(), &args)
    };

    let out = run(&[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(exists(&name));

    // Same name again: refused without --replace.
    let again = run(&[]);
    assert!(!again.status.success());
    assert!(String::from_utf8_lossy(&again.stderr).contains("--replace"));

    let ps = dkr(dir.path(), &["--ps"]);
    assert!(stdout(&ps).contains(&name), "{}", stdout(&ps));

    wait_until("log output", || {
        stdout(&dkr(dir.path(), &[profile.as_str(), "--logs"])).contains("started")
    });

    let replaced = run(&["--replace"]);
    assert!(
        replaced.status.success(),
        "{}",
        String::from_utf8_lossy(&replaced.stderr)
    );

    // --init means Ctrl-C (SIGINT) actually stops the container.
    let _ = docker(&["kill", "-s", "INT", &name]);
    wait_until("container to stop on SIGINT", || !exists(&name));

    let out = run(&[]);
    assert!(out.status.success());
    let stopped = dkr(dir.path(), &[profile.as_str(), "--stop"]);
    assert!(
        stopped.status.success(),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    assert!(!exists(&name));
}
