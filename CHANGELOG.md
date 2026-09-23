# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/) (loose
semver pre-1.0).

## [Unreleased]

## [0.1.0]

- Initial release: `dkr <profile>` runs a one-off `docker run --rm` for a
  named image profile (volumes, env, workdir/user, network/ports, entrypoint
  override, raw extra args), with `--list` and `--dry-run`.
