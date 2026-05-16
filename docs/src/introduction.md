# Introduction

`rusta` is a macOS-only CLI for creating and managing Ubuntu VMs on Apple
Silicon using [Tart](https://tart.run/).

It is the spiritual successor to `ubuntu-tart-vm.sh`, exposing its features
through a subcommand-based UX rather than a single mega-script. It handles
cloning Ubuntu OCI images, provisioning guests over SSH, wiring up Docker,
and keeping track of a default VM so day-to-day commands stay short.

## Requirements

- Apple Silicon Mac (`arm64`). The CLI aborts at startup on any other
  architecture.
- macOS with [Homebrew](https://brew.sh/) available on `PATH`.
- Outbound HTTPS to `ghcr.io` for OCI image pulls and `rusta versions`.

`tart`, `sshpass`, and (only for `docker-setup`) `docker` are auto-installed
via Homebrew on demand.

## Where next?

- [Installation](installation.md) — install via Homebrew, crates.io, or from
  source.
- [Quick Start](quick-start.md) — the shortest path from zero to an SSH
  session in a fresh Ubuntu VM.
- [Commands](commands.md) — full CLI command reference.
