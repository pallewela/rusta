# Installation

## Homebrew (recommended)

```sh
brew install pallewela/tap/rusta
```

This installs the latest prebuilt `aarch64-apple-darwin` binary from the
[GitHub Releases](https://github.com/pallewela/rusta/releases) page.
Upgrades land via `brew upgrade rusta`.

## Manual download

Grab the tarball for the release you want from
[GitHub Releases](https://github.com/pallewela/rusta/releases), then:

```sh
tar -xzf rusta-vX.Y.Z-aarch64-apple-darwin.tar.gz
install -m 0755 rusta /usr/local/bin/rusta
```

Each release page lists the SHA256 of the tarball; verify with
`shasum -a 256` before installing.

## From crates.io

```sh
cargo install rusta-cli
```

The crate is named `rusta-cli` on crates.io because `rusta` was already
taken; the installed binary is still `rusta`.

## From source

```sh
cargo install --path .
# or
cargo install --git https://github.com/pallewela/rusta
```
