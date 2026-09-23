# Agent Instructions

`dkr` is a thin wrapper around `docker run`: it applies a named per-image
YAML profile (volumes, env, workdir/user, network/ports, entrypoint,
`--init`, raw extra args) so a one-off command from an image doesn't require
retyping `docker run` flags. See `README.md` for the user-facing overview.

## Build, test, verify

- `make check` is the one gate to run before committing or reporting work
  done. It runs `cargo fmt --check`, `cargo check`, `cargo test`, and
  `make schema-check`.
- `make help` lists every target.
- `cargo run -- <profile> --dry-run` prints the constructed docker command
  without executing it — the fastest way to inspect behavior without
  touching real containers.
- Docker-dependent behavior (signal handling, actual container execution)
  has no CI-level integration test yet (see Known Gaps below); spot-check it
  manually against real docker when changing `src/docker_cmd.rs` or
  anything touching how the process is spawned.

## Architecture

- `src/cli.rs` — the `clap` derive `Cli` struct. Flags only, no subcommands
  (see Conventions below).
- `src/config.rs` — `Profile`/`Volume` structs (`serde` + `schemars`),
  config directory resolution, `list_profiles`, `schema_json`, `validate`.
- `src/docker_cmd.rs` — `build_args()`: a pure function, `Profile` + trailing
  command → docker run argv. This is the main unit-tested surface; no
  docker execution is needed to test it.
- `src/main.rs` — thin wiring: parse CLI, then branch to `--schema` /
  `--list` / `--validate` / `--dry-run` / actually exec `docker`.
- `src/lib.rs` — re-exports the modules so integration tests under `tests/`
  can use them as `dkr::config::...` etc.

## Conventions

- Flat `src/` (one file per concern), no `cmd/`/`internal/` split — matches
  this user's other personal Rust projects.
- The CLI is flags only, deliberately never subcommands: a profile could be
  named anything, so a reserved verb like `list` or `schema` would risk
  colliding with a real profile name. `--list`, `--schema`, `--validate`,
  `--dry-run` are all flags for this reason.
- Always shell out to the real `docker` binary (`std::process::Command`)
  rather than talking to the Docker Engine API directly. `docker run -it`
  already handles pty allocation, raw terminal mode, resize forwarding, and
  signal proxying correctly; reimplementing that against the Engine API
  would mean rebuilding a meaningful slice of the docker CLI itself. This is
  also why the `--init` fix (see Known Gotchas) was a one-line flag instead
  of custom signal-handling code.
- `Profile`/`Volume` use `#[serde(deny_unknown_fields)]` — a typo'd YAML key
  is a hard parse error (listing valid field names), not a silently ignored
  no-op.
- `Profile`/`Volume` derive `schemars::JsonSchema`, and every field has a
  doc comment. Those doc comments become the JSON Schema `description` shown
  to editors and any other tool reading `schema.json` — keep them accurate.

## When you change the `Profile` or `Volume` struct

1. Add/edit the field with an accurate doc comment.
2. Run `make schema` to regenerate `schema.json`.
3. `make check` fails via `schema-check` if you forget step 2 — it diffs a
   freshly generated schema against the committed one.
4. Update `README.md`'s field reference and `examples/example.yaml` if the
   change is user-facing.
5. Add or update a fixture under `tests/fixtures/` and a case in
   `tests/docker_cmd.rs` or `tests/config.rs` covering the new behavior.

## Releasing

1. Bump `version` in `Cargo.toml`, add a `CHANGELOG.md` entry under a new
   version heading, `cargo build` once so `Cargo.lock` picks up the version,
   commit.
2. `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. CI verifies the tag matches `Cargo.toml`'s version, builds binaries for
   linux/macos × x86_64/arm64, creates the GitHub Release with those
   binaries attached, and publishes to crates.io (needs the
   `CARGO_REGISTRY_TOKEN` repository secret).
4. To validate the pipeline without releasing anything, run the `Publish`
   workflow via `workflow_dispatch` with `dry_run: true`.

Loose semver pre-1.0: a behavior change (like `deny_unknown_fields`) bumps
the minor version even though it's not a Rust API break; a straightforward
bug fix bumps the patch version.

## Known gotchas

- Without `--init`, a containerized process running as PID 1 doesn't get
  default signal handling from the kernel — Ctrl-C silently does nothing.
  `dkr` always passes `--init` (opt out per-profile with `init: false`) for
  exactly this reason. Verify empirically, not just by reasoning about it:
  `docker run -d --rm --name t alpine sleep 300; docker kill -s INT t;
  docker ps` (still running = bug) vs. the same with `--init` added
  (stops = fixed).
- GitHub Actions secrets are write-only — there is no way to read
  `CARGO_REGISTRY_TOKEN`'s value back later, even via the API, even for a
  repo admin. If it's ever needed on a new repo, it has to come from
  crates.io or wherever it was originally stored, not copied cross-repo.

## Known gaps (candidates for future work, not yet done)

- No docker-integration test runs in CI — everything is tested as pure
  argv-building. The `--init` bug above was caught by manual testing, not
  the test suite. A `#[ignore]`-gated test that really shells out to docker
  (GitHub-hosted runners have it preinstalled) would close this.
- `validate()` only runs when `--validate` is passed; a normal run with a
  malformed `ports` entry just fails inside docker with a less clear error.
- No no-profile "quick" mode (e.g. `dkr --image alpine -- echo hi`) for a
  genuine one-off without a saved profile file first.
- No shell completions (`clap_complete` would make this cheap).
