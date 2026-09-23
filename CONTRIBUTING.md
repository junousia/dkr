# Contributing

Bug reports, feature requests, and pull requests are welcome via GitHub
issues/PRs on this repository.

## Development

See [`AGENTS.md`](AGENTS.md) for the full architecture map, conventions, and
release process. In short:

```bash
make check   # fmt check, cargo check, tests, schema.json freshness
make help    # list all developer commands
```

Before opening a PR:

- `make check` passes.
- If you changed `Profile`/`Volume` in `src/config.rs`, run `make schema`
  and include the updated `schema.json`.
- Add a `CHANGELOG.md` entry under `[Unreleased]`.
- Add or update tests under `tests/` for the behavior you changed —
  `tests/docker_cmd.rs` for anything affecting the constructed `docker run`
  command, `tests/config.rs` for schema/validation behavior.

## Reporting security issues

See [`SECURITY.md`](SECURITY.md).
