# Contributing to rusta

Thanks for your interest in `rusta`. This document covers how to set up
a development environment, the conventions the project follows, and how
to get a change merged.

For **bug reports**, **feature requests**, and other support questions,
see [SUPPORT.md](SUPPORT.md). For **security vulnerabilities**, see
[SECURITY.md](SECURITY.md) — please do not file them as public issues.

## Quick start

```sh
git clone https://github.com/pallewela/rusta.git
cd rusta
cargo build
cargo test
```

The integration tests under `tests/` exercise the CLI surface
end-to-end against stubbed `tart`/`brew`/`ssh` binaries, so they run on
any platform that supports a Rust toolchain — you do not need an actual
Tart install to develop.

## How to submit a change

1. Fork the repository on GitHub.
2. Create a feature branch: `git checkout -b feat/short-description`.
3. Make your changes. Run `cargo build`, `cargo test`, and `cargo fmt`
   before committing.
4. Commit using [Conventional Commits](https://www.conventionalcommits.org/)
   — the auto-generated CHANGELOG keys off the commit prefix:
   - `feat: ...` — new feature
   - `fix: ...` — bug fix
   - `docs: ...` — documentation only
   - `test: ...` — test-only change
   - `ci: ...` — CI/release pipeline change
   - `refactor: ...` — code change that neither fixes a bug nor adds a feature
   - `chore: ...` — anything else
5. Push the branch and open a pull request against `main`.

## What happens after you open a PR

- **CI** runs on macOS arm64 (build, integration tests with coverage,
  cargo-deny advisories/licenses).
- **CodeQL** analyzes the Rust source and the workflow YAMLs.
- For PRs by the maintainer or by Dependabot, the
  [`auto-approve` workflow](.github/workflows/auto-approve.yml) approves
  the PR once CI passes and enables auto-merge. Squash-merge happens as
  soon as every required gate is green.
- For PRs by external contributors, the workflow does **not**
  auto-approve. The maintainer will review by hand; expect a response
  within ~7 days. Mention `@pallewela` in the PR description if you
  need an earlier look.

After merge, the existing automation handles tagging, release notes,
CHANGELOG update, GitHub Release with SBOM + SLSA provenance,
crates.io publish, and the Homebrew tap bump.

## Coding conventions

- **Rust edition 2021.** The `Cargo.toml` `edition` field is the source
  of truth.
- **Formatting.** Run `cargo fmt` before committing. CI does not yet
  fail on `rustfmt`, but please keep diffs clean.
- **Lints.** `cargo clippy --all-targets -- -W clippy::all` is a useful
  pre-commit check.
- **Tests.** Add or extend an integration test under `tests/` for any
  user-facing behaviour change. Unit tests live next to the code under
  `#[cfg(test)] mod tests {}`.
- **Comments.** Default to writing no comments. Explain *why*, not
  *what*, when a comment is necessary. Don't document the obvious.
- **Dependencies.** Avoid adding new crates unless the alternative is
  meaningfully worse. New dependencies go through `cargo-deny`'s license
  and advisory checks in CI.

## Project structure

```
src/                # CLI entry point and command implementations
src/commands/       # One module per subcommand (up, down, list, …)
src/tart.rs         # Wrapper around the `tart` binary
src/state.rs        # Persisted CLI state (default VM, etc.)
tests/              # Integration tests using a stubbed environment
.github/workflows/  # CI, release, security, and Scorecard workflows
docs/               # mdBook documentation source (also published as a site)
```

## License

By contributing, you agree that your contributions will be licensed
under the [MIT License](LICENSE), the same license as the project.
