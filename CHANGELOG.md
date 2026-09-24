# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/) (loose
semver pre-1.0).

## [Unreleased]

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
