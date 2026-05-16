# rusta

[![CI](https://github.com/pallewela/rusta/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/pallewela/rusta/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/pallewela/rusta/branch/main/graph/badge.svg)](https://codecov.io/gh/pallewela/rusta)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/pallewela/rusta/badge)](https://scorecard.dev/viewer/?uri=github.com/pallewela/rusta)
[![Release](https://img.shields.io/github/v/release/pallewela/rusta?sort=semver)](https://github.com/pallewela/rusta/releases/latest)
[![Last release](https://img.shields.io/github/release-date/pallewela/rusta)](https://github.com/pallewela/rusta/releases/latest)
[![License](https://img.shields.io/github/license/pallewela/rusta)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20arm64-blue)](#requirements)
[![Homebrew tap](https://img.shields.io/badge/brew-pallewela%2Ftap%2Frusta-orange?logo=homebrew)](https://github.com/pallewela/homebrew-tap)

A macOS-only CLI for creating and managing Ubuntu VMs on Apple Silicon using [Tart](https://tart.run/).

`rusta` is the spiritual successor to `ubuntu-tart-vm.sh`, exposing its features through a subcommand-based UX rather than a single mega-script. It handles cloning Ubuntu OCI images, provisioning guests over SSH, wiring up Docker, and keeping track of a default VM so day-to-day commands stay short.

## Requirements

- Apple Silicon Mac (`arm64`). The CLI aborts at startup on any other architecture.
- macOS with [Homebrew](https://brew.sh/) available on `PATH`.
- Outbound HTTPS to `ghcr.io` for OCI image pulls and `rusta versions`.

`tart`, `sshpass`, and (only for `docker-setup`) `docker` are auto-installed via Homebrew on demand.

## Install

### Homebrew (recommended)

```sh
brew install pallewela/tap/rusta
```

This installs the latest prebuilt `aarch64-apple-darwin` binary from the
[GitHub Releases](https://github.com/pallewela/rusta/releases) page.
Upgrades land via `brew upgrade rusta`.

### Manual download

Grab the tarball for the release you want from
[GitHub Releases](https://github.com/pallewela/rusta/releases), then:

```sh
tar -xzf rusta-vX.Y.Z-aarch64-apple-darwin.tar.gz
install -m 0755 rusta /usr/local/bin/rusta
```

Each release page lists the SHA256 of the tarball; verify with
`shasum -a 256` before installing.

### From crates.io

```sh
cargo install rusta-cli
```

The crate is named `rusta-cli` on crates.io because `rusta` was already
taken; the installed binary is still `rusta`.

### From source

```sh
cargo install --path .
# or
cargo install --git https://github.com/pallewela/rusta
```

## Releasing

Maintainer notes for cutting a new release:

1. Bump `version` in `Cargo.toml` (e.g. `0.2.0`), commit as `chore: release v0.2.0`.
2. `git tag v0.2.0 && git push --tags`.
3. The `Release` workflow (`.github/workflows/release.yml`) builds on
   `macos-latest`, attaches `rusta-v0.2.0-aarch64-apple-darwin.tar.gz` to a
   GitHub Release, and (when the `TAP_REPO_TOKEN` secret is configured)
   dispatches a `rusta-release` event to `pallewela/homebrew-tap` so the
   Formula picks up the new version and sha256 automatically.

## Quick start

```sh
# Create + provision a default Ubuntu 24.04 VM
rusta create

# Boot it (headless by default)
rusta up

# SSH in
rusta ssh

# Find its IP
rusta ip

# Shut it down gracefully
rusta down
```

The first time you run an argument-less command with more than one VM present, `rusta` interactively prompts you to pick a default and persists the choice to `~/.local/share/rusta/state.toml`.

## Commands

| Subcommand                     | Purpose                                                          |
| ------------------------------ | ---------------------------------------------------------------- |
| `rusta up [<vm>]`              | Start a VM (headless by default; `--graphical` for a window).    |
| `rusta down [<vm>]`            | Gracefully shut down a VM (`--force` to hard-stop).              |
| `rusta create [<vm>]`          | Create and provision a new Ubuntu VM.                            |
| `rusta delete <vm>`            | Delete a VM (requires confirmation or `--yes`).                  |
| `rusta list`                   | List Tart VMs and indicate the current default.                  |
| `rusta versions`               | List available Ubuntu OCI tags from `ghcr.io/cirruslabs/ubuntu`. |
| `rusta default [<vm>]`         | Print or set the default VM.                                     |
| `rusta ip [<vm>]`              | Print the guest IP of the VM.                                    |
| `rusta ssh [<vm>] [-- cmd...]` | Open an SSH session or run a command on the VM.                  |
| `rusta docker-setup [<vm>]`    | Install Docker in the VM and wire host SSH/Docker context.       |
| `rusta ssh-copy [<vm>]`        | Copy host `~/.ssh/id_*` and `*.pem` into the VM.                 |

Global flags accepted by every subcommand:

- `--verbose` — verbose logging.
- `--log <file>` — tee all stdout/stderr to the given file.
- `--help`, `-h` — print help and exit.

For full behavior, flags, and exit codes, see [`SPEC.md`](SPEC.md).

## State

`rusta` keeps a small amount of host-side state under `~/.local/share/rusta/`:

- `state.toml` — the default VM name.
- `run/<vm>.pid` — PID of the headless `tart run` process.
- `provision/<vm>.sh` — the generated provisioning script (kept for debugging).

## Development

```sh
cargo build
cargo test
```

The integration tests under `tests/` exercise the CLI surface end-to-end against stubbed `tart`/`brew`/`ssh` binaries, so they run without an actual Tart install.

## License

MIT. See [LICENSE](LICENSE).
