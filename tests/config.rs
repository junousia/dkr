use std::path::{Path, PathBuf};

use dkr::config::{schema_json, validate, Profile};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load(name: &str) -> Profile {
    Profile::load(&fixtures_dir(), name).expect("fixture should parse")
}

#[test]
fn schema_is_valid_json_and_describes_profile() {
    let schema = schema_json();
    let value: serde_json::Value = serde_json::from_str(&schema).expect("schema is valid JSON");
    assert_eq!(value["title"], "Profile");
    assert_eq!(value["type"], "object");
    assert_eq!(value["required"][0], "image");
    assert!(value["properties"]["image"].is_object());
    assert!(value["properties"]["volumes"].is_object());
}

#[test]
fn unknown_field_is_rejected_at_parse_time() {
    let dir = tempfile::tempdir().expect("create temp dir");
    std::fs::write(
        dir.path().join("typo.yaml"),
        "image: alpine:latest\nenviroment:\n  FOO: bar\n",
    )
    .expect("write fixture");

    let err = Profile::load(dir.path(), "typo").unwrap_err();
    assert!(err.to_string().contains("failed to parse"));
    let source = std::error::Error::source(&err).expect("has source");
    assert!(source.to_string().contains("enviroment"));
}

#[test]
fn validate_accepts_a_well_formed_profile() {
    let profile = load("full");
    assert_eq!(validate(&profile), Vec::<String>::new());
}

#[test]
fn validate_reports_every_problem() {
    let dir = tempfile::tempdir().expect("create temp dir");
    std::fs::write(
        dir.path().join("bad.yaml"),
        r#"
image: alpine:latest
ports:
  - "notaport"
extra_args:
  - ""
volumes:
  - host: ""
    container: /workspace
"#,
    )
    .expect("write fixture");

    let profile = Profile::load(dir.path(), "bad").expect("fixture should parse");
    let problems = validate(&profile);

    assert_eq!(problems.len(), 3);
    assert!(problems.iter().any(|p| p.contains("volumes[0].host")));
    assert!(problems.iter().any(|p| p.contains("ports[0]")));
    assert!(problems.iter().any(|p| p.contains("extra_args[0]")));
}
