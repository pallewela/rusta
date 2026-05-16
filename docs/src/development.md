# Development

```sh
cargo build
cargo test
```

The integration tests under `tests/` exercise the CLI surface end-to-end
against stubbed `tart`/`brew`/`ssh` binaries, so they run without an
actual Tart install.

## Releasing

Maintainer notes for cutting a new release:

1. Bump `version` in `Cargo.toml` (e.g. `0.2.0`), commit as
   `chore: release v0.2.0`.
2. `git tag v0.2.0 && git push --tags`.
3. The `Release` workflow (`.github/workflows/release.yml`) builds on
   `macos-latest`, attaches `rusta-v0.2.0-aarch64-apple-darwin.tar.gz` to
   a GitHub Release, and (when the `TAP_REPO_TOKEN` secret is configured)
   dispatches a `rusta-release` event to `pallewela/homebrew-tap` so the
   Formula picks up the new version and sha256 automatically.
