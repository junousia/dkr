#!/bin/sh
# Installs the dkr binary from the latest GitHub Release.
#
#   curl -fsSL https://raw.githubusercontent.com/junousia/dkr/main/install.sh | sh
#
# Environment:
#   DKR_VERSION      release to install, e.g. v0.5.0 (default: latest)
#   DKR_BASE_URL     download from here instead of GitHub (mirrors, testing)
#   DKR_INSTALL_DIR  where to put the binary (default: $HOME/.local/bin)
set -eu

REPO="junousia/dkr"

say() { printf 'dkr-install: %s\n' "$*"; }
die() { printf 'dkr-install: error: %s\n' "$*" >&2; exit 1; }

fetch() { # fetch URL DEST
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$1" -O "$2"
    else
        die "need curl or wget"
    fi
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d' ' -f1
    else
        return 1
    fi
}

main() {
    case "$(uname -s)" in
        Linux) os=linux ;;
        Darwin) os=macos ;;
        *) die "unsupported OS '$(uname -s)'; try: cargo install dkr" ;;
    esac
    case "$(uname -m)" in
        x86_64 | amd64) arch=x86_64 ;;
        aarch64 | arm64) arch=arm64 ;;
        *) die "unsupported architecture '$(uname -m)'; try: cargo install dkr" ;;
    esac
    name="dkr-$os-$arch"

    if [ -n "${DKR_BASE_URL:-}" ]; then
        base="${DKR_BASE_URL%/}"
    elif [ -n "${DKR_VERSION:-}" ]; then
        base="https://github.com/$REPO/releases/download/v${DKR_VERSION#v}"
    else
        base="https://github.com/$REPO/releases/latest/download"
    fi
    dir="${DKR_INSTALL_DIR:-$HOME/.local/bin}"

    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT INT TERM

    say "downloading $name (${DKR_VERSION:-latest})"
    fetch "$base/$name" "$tmp/$name" || die "download failed: $base/$name"

    if fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS" 2>/dev/null; then
        expected="$(grep " $name\$" "$tmp/SHA256SUMS" | cut -d' ' -f1)"
        actual="$(sha256 "$tmp/$name")" || die "need sha256sum or shasum to verify the download"
        [ -n "$expected" ] || die "$name not listed in SHA256SUMS"
        [ "$expected" = "$actual" ] || die "checksum mismatch for $name"
        say "checksum verified"
    else
        say "warning: this release has no SHA256SUMS; skipping checksum verification"
    fi

    mkdir -p "$dir"
    chmod 755 "$tmp/$name"
    mv "$tmp/$name" "$dir/dkr"
    say "installed $("$dir/dkr" --version) to $dir/dkr"

    case ":$PATH:" in
        *":$dir:"*) ;;
        *) say "note: $dir is not on your PATH; add: export PATH=\"$dir:\$PATH\"" ;;
    esac
}

# Wrapped in a function so a truncated download can't run a partial script.
main "$@"
