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

The tarball also ships shell completions and man pages under `share/`.
Install them alongside the binary:

```sh
# man pages
sudo install -d /usr/local/share/man/man1
sudo install -m 0644 share/man/man1/*.1 /usr/local/share/man/man1/

# zsh completions (adjust to your $fpath)
sudo install -m 0644 share/zsh/site-functions/_rusta /usr/local/share/zsh/site-functions/

# bash / fish — install to your shell's standard completion path
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

## Shell completions

If you installed via `cargo install` or built from source, the prebuilt
completion scripts aren't on disk — generate them yourself at any time:

```sh
rusta completions bash > /usr/local/etc/bash_completion.d/rusta
rusta completions zsh  > "${fpath[1]}/_rusta"
rusta completions fish > ~/.config/fish/completions/rusta.fish
```

Homebrew installs and the Manual download tarball already include these
under `share/`, so no extra step is needed there.
