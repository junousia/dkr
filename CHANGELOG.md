# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/) (loose
semver pre-1.0).

## [Unreleased]

### Changed

- Linux release binaries are now statically linked musl builds, so they no
  longer require a recent glibc (previously glibc 2.39).

## [0.5.1]

### Added

- `install.sh` for `curl ... | sh` installs of the prebuilt release binary;
  releases now publish a `SHA256SUMS` file it verifies against.

## [0.5.0]

### Added

- Quick mode: `dkr --image alpine -- echo hi` runs without a profile file.
- Per-run overrides: `-e`, `-v`, `-p`, `-w`, `-u`, `--network`,
  `--entrypoint`, `--platform`, `--pull`, `--mount-cwd`, and `-a` for any raw
  docker flag.
- `extends: <profile>` profile inheritance.
- New profile fields: `mount_cwd`, `env_file`, `pull`, `platform`,
  `hostname`, `add_hosts`, `tty`.
- `$(command)`, `~` and `$VAR` expansion now applies to every string field,
  not just volume hosts and env values.
- `dkr --completions <shell>`.

### Changed

- Profile sanity checks (`validate`) now run before every normal run, not
  only under `--validate`, and also check `$(` is closed in every field.
- `ports` validation accepts every docker form (`80`, `ip:host:container`,
  `80/udp`, ranges) instead of requiring `host:container`.

## [0.4.0]

### Added

- Optional `description` profile field and `dkr --list --json` (name,
  description, image, error per profile) for scripts and agents.
- `--help` footer and a README section on using dkr from agents.
- `$(command)` substitution in `volumes[].host` and `env` values.

### Changed

- `docker_cmd::build_args` no longer expands `~`/`$VAR`; call
  `Profile::resolve()` first.

## [0.3.0]

### Added

- `dkr <profile>:<tag>` overrides the profile's image tag (or digest) for a
  single run, e.g. `dkr myimage:1.0.0 -- echo hello`, without editing the
  profile file. Preserves any registry, including one with a port.

## [0.2.0]

### Added

- `dkr --schema` prints a JSON Schema for the profile format, derived
  directly from the `Profile` struct and checked into the repo as
  `schema.json` (kept honest by `make schema-check`, part of `make check`).
  Useful standalone for editor autocomplete/validation or other tooling.
- `dkr <profile> --validate` checks a profile without running docker.

### Changed

- Unknown fields in a profile YAML are now rejected at parse time (with a
  message listing valid field names), instead of being silently ignored.
  This can break a profile that previously had an unrecognized field.

## [0.1.1]

### Fixed

- Containers now always run with `--init` (opt out with `init: false` in a
  profile). Without it, a containerized command running as PID 1 (e.g.
  `dkr myimage -- watch ls`) doesn't get default signal handling, so Ctrl-C
  had no effect and the container kept running.

## [0.1.0]

- Initial release: `dkr <profile>` runs a one-off `docker run --rm` for a
  named image profile (volumes, env, workdir/user, network/ports, entrypoint
  override, raw extra args), with `--list` and `--dry-run`.
