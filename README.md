# dkr

Run one-off commands from docker images using named per-image profiles, so
you never have to retype `docker run` flags by hand.

```bash
dkr myimage                       # run the image's default command
dkr myimage -- bash                # run bash inside the container instead
dkr myimage -- ./scripts/build.sh   # run a script inside the container
dkr myimage:1.0.0 -- echo hello      # override the profile's image tag for this run
dkr --image alpine -- echo hi       # quick mode: no profile file needed
dkr myimage -e DEBUG=1 -p 9000      # per-run env / mount / port overrides
dkr --list                         # show available profiles
dkr myimage --dry-run               # print the docker command without running it
```

`dkr` always runs containers with `--rm` (ephemeral, one-off) and `--init`
(so the containerized command isn't PID 1 and gets normal signal handling —
Ctrl-C actually works), and adds `-it` automatically when stdin is a
terminal. It is not for long-running services — see `docker compose` for
that.

## Install

Prebuilt binary (Linux/macOS, x86_64/arm64) into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/junousia/dkr/main/install.sh | sh
```

Set `DKR_VERSION=v0.5.0` to pin a release or `DKR_INSTALL_DIR=/usr/local/bin`
to change the destination. The script verifies the download against the
release's `SHA256SUMS` when one is published (releases after 0.5.0).

Build from source:

```bash
cargo install --path .
```

Or from crates.io:

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

Only `image` is required. Other optional fields not shown above:

| Field | Docker flag | Notes |
| --- | --- | --- |
| `description` | — | One-line summary shown by `--list --json` |
| `extends` | — | Inherit from another profile (see below) |
| `mount_cwd` | `-v $PWD:<path>` | Mounts the directory you run `dkr` from; also sets `workdir` unless given |
| `env_file` | `--env-file` | List of files |
| `pull` | `--pull` | `always`, `missing` or `never` |
| `platform` | `--platform` | e.g. `linux/amd64` |
| `hostname` | `--hostname` | |
| `add_hosts` | `--add-host` | List of `host:ip` |
| `tty` | `-it` | `true`/`false` forces or suppresses it; default is auto-detect |

Every string field (image, volumes, env, workdir, user, network, ports,
entrypoint, extra_args, …) is expanded, so `~` and `$VAR`/`${VAR}`
(including `${PWD}`) work as written above.

They also support command substitution: `$(command)` is run with `sh -c` and
replaced by its stdout (trailing newlines trimmed), e.g.
`API_TOKEN: "$(pass show api/token)"`. A failing command aborts the run.
Output is inserted verbatim, never re-expanded; write `$$(` for a literal
`$(`. Commands also run for `--dry-run` (so the printed command is the real
one, secrets included), but never for `--validate`.

Running `dkr myimage` then applies all of that automatically. See
[`examples/example.yaml`](examples/example.yaml) for a copy-pasteable
starting point.

### Inheriting from another profile

`extends: <profile>` starts from another profile in the same directory:

```yaml
# ~/.config/dkr/base.yaml
image: myorg/base:1
user: "1000:1000"
volumes:
  - { host: "${PWD}", container: /workspace }
env: { TZ: UTC }
```

```yaml
# ~/.config/dkr/rust.yaml
extends: base
image: rust:1
env: { CARGO_HOME: /cache }
```

Scalars in the child replace the parent's; `env` is merged (child wins);
`volumes`, `ports`, `extra_args`, `env_file` and `add_hosts` are appended
after the parent's, except that a child volume replaces a parent volume with
the same `container` path. Chains and cycle detection are supported. A base
profile needs an `image` too (or `--list --json` reports it as an error).

### Per-run overrides and quick mode

Flags after the profile name adjust a single run without editing the file:

```bash
dkr myimage -e DEBUG=1 -e HOME_TOKEN        # KEY=VALUE, or KEY to pass the host value through
dkr myimage -v ~/data:/data:ro -p 9000:9000  # replaces a profile mount at the same container path
dkr myimage --mount-cwd /src -w /src/app     # also -u, --network, --entrypoint, --platform, --pull
dkr myimage -a --cap-add=SYS_PTRACE          # any other raw docker flag, repeatable
```

Override values are used verbatim (your shell already expanded them).

`--image <image>` runs with no profile file at all; every positional
argument is then part of the command, and all the flags above still apply:

```bash
dkr --image alpine:3 --mount-cwd /work -- ls -la
```

### Shell completions

```bash
dkr --completions zsh  > ~/.zfunc/_dkr          # also bash, fish, elvish, powershell
dkr --completions bash > ~/.local/share/bash-completion/completions/dkr
```

Completions cover flags; profile names are not completed.

### Overriding the image tag

Append `:tag` to the profile name to run a different version of the image
for one invocation, without editing the profile file:

```bash
dkr myimage:1.0.0 -- echo hello
```

This replaces the tag (or digest) on the profile's `image`, preserving any
registry, including one with a port (e.g. `localhost:5000/myimage:latest`
becomes `localhost:5000/myimage:1.0.0`).

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
names), and the same sanity checks (empty paths, malformed `ports`, unclosed
`$(`) run before every normal run, so a bad profile fails with a clear message
instead of a docker error. `--validate` runs them without running docker.

## Using dkr from an agent or script

Everything an agent needs is discoverable from the binary itself, and none
of these steps touch docker (except as noted):

```bash
dkr --list --json          # [{name, description, image, error}, ...]
dkr --schema               # JSON Schema of the profile format
dkr myimage --validate     # check a profile (runs no commands)
dkr myimage --dry-run      # print the docker command (runs $(...) values)
```

Give each profile a `description:` so `--list --json` explains what it is
for. A profile that fails to parse is reported with an `error` instead of
being omitted. Exit codes: `0` success, non-zero on any error, and a real run
exits with the container's own exit code.

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
