# dkr

Run one-off commands from docker images using named per-image profiles, so
you never have to retype `docker run` flags by hand.

```bash
dkr myimage                       # run the image's default command
dkr myimage -- bash                # run bash inside the container instead
dkr myimage -- ./scripts/build.sh   # run a script inside the container
dkr --list                         # show available profiles
dkr myimage --dry-run               # print the docker command without running it
```

`dkr` always runs containers with `--rm` (ephemeral, one-off) and `--init`
(so the containerized command isn't PID 1 and gets normal signal handling —
Ctrl-C actually works), and adds `-it` automatically when stdin is a
terminal. It is not for long-running services — see `docker compose` for
that.

## Install

Build from source:

```bash
cargo install --path .
```

Or, once published:

```bash
cargo install dkr
```

## Configuring a profile

Each profile is a YAML file named `<profile>.yaml` under the config
directory, resolved in this order: `--config-dir`, `$DKR_CONFIG_DIR`, then
`~/.config/dkr`.

For example, `~/.config/dkr/myimage.yaml`:

```yaml
image: myorg/myimage:latest
volumes:
  - host: "${PWD}"
    container: /workspace
  - host: ~/cache
    container: /cache
    options: ro          # optional, e.g. "ro"
env:
  FOO: bar
init: true                # optional, default true; passes docker's --init
workdir: /workspace
user: "1000:1000"
network: host             # optional
ports:
  - "8080:80"             # optional, "host:container"
entrypoint: /bin/bash      # optional, overrides the image's entrypoint
extra_args:
  - "--cap-add=SYS_PTRACE" # optional, raw extra docker run flags
```

Only `image` is required. Host paths and env values are expanded, so `~` and
`$VAR`/`${VAR}` (including `${PWD}`) work as written above.

Running `dkr myimage` then applies all of that automatically. See
[`examples/example.yaml`](examples/example.yaml) for a copy-pasteable
starting point.

## Schema and validation

`dkr --schema` prints a JSON Schema for the profile format, derived directly
from the code so it can't drift out of sync. It's self-contained (no need to
clone this repo) and useful for editor autocomplete/validation, or for any
other tool or agent that wants to generate or check a profile:

```bash
dkr --schema > ~/.config/dkr/schema.json
```

Then reference it from a profile file for editor support (VS Code/JetBrains
with a YAML language server extension):

```yaml
# yaml-language-server: $schema=./schema.json
image: myorg/myimage:latest
```

`dkr <profile> --validate` checks a profile without running docker: unknown
fields are rejected at parse time (with a message listing the valid field
names), and `--validate` additionally checks for common mistakes like empty
paths or malformed `ports` entries.

## Verify

```bash
make check
```

List available developer workflows:

```bash
make help
```

## Releasing

CI runs `make check` on every push and pull request. Pushing a tag matching
`v*.*.*` (the version must match `Cargo.toml`) triggers the publish workflow:
it builds binaries for Linux (x86_64/arm64) and macOS (x86_64/arm64), creates
a GitHub Release with those binaries attached, and publishes the crate to
crates.io.

```bash
git tag v0.1.0
git push origin v0.1.0
```

Use `workflow_dispatch` with `dry_run: true` on the `Publish` workflow to
validate the build/package steps without releasing or publishing anything.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the development workflow and
[`AGENTS.md`](AGENTS.md) for the fuller architecture/conventions map (useful
for AI coding agents and humans alike). Security issues: see
[`SECURITY.md`](SECURITY.md).
