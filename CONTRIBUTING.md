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

## Versioning & releases

- **Versioning scheme:** Semantic Versioning 2.0.0
  ([semver.org](https://semver.org/)) — `MAJOR.MINOR.PATCH`.
- **Release tags:** every release is identified in version control by
  an annotated git tag named `vMAJOR.MINOR.PATCH` (e.g. `v1.0.16`).
  The tag set on `main` is the source of truth for the list of
  releases the project has ever cut — list them with
  `git tag --list 'v*' --sort=-v:refname`.
- **Publication channels for each tag:**
  - A [GitHub Release](https://github.com/pallewela/rusta/releases)
    with the `aarch64-apple-darwin` tarball, a CycloneDX SBOM, and a
    Sigstore-signed SLSA provenance attestation.
  - A [crates.io](https://crates.io/crates/rusta-cli) version of
    `rusta-cli`.
  - A [Homebrew tap](https://github.com/pallewela/homebrew-tap)
    formula bump.
- **Tagging is fully automated.** Maintainers do not create tags by
  hand. The [`Auto-tag` workflow](.github/workflows/auto-tag.yml)
  runs after CI succeeds on `main` and:
  1. Inspects the head commit message to pick the SemVer bump level
     — `[bump:major]` / `BREAKING CHANGE` → major; `[bump:minor]` →
     minor; default → patch; `[skip release]` → no tag.
  2. Regenerates `CHANGELOG.md` via `git-cliff`.
  3. Commits the updated CHANGELOG and pushes a new annotated tag
     pointing at that commit.
- **Tag immutability policy.** Once pushed, release tags are
  considered immutable: they are not deleted, moved, or rewritten.
  The repository enforces this with a `protect-tags` ruleset on
  `refs/tags/v*` (`deletion` and `non_fast_forward` blocked). If a
  release contains a defect, the fix ships as a new patch release
  with an incremented tag, never as a rewritten old tag.

## Requirements for acceptable contributions

This section uses [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119)
language. **MUST** items are hard requirements — CI enforces them and
a PR cannot merge without satisfying every one. **SHOULD** items are
expectations during review; deviating from one is fine if you have a
reason.

### MUST (enforced by CI)

1. **The change builds.** `cargo build --all-targets --locked`
   succeeds on `macos-latest` (Apple Silicon).
2. **All tests pass.** `cargo llvm-cov --locked --lcov` succeeds; CI
   uploads the lcov to Codecov.
3. **No new advisories or banned crates.**
   `cargo deny --locked check` (advisories, licenses, bans, sources)
   exits clean. New dependencies must use a license on
   `deny.toml`'s allow-list — typically MIT / Apache-2.0 / BSD /
   ISC / Unicode / CDLA-Permissive. Copyleft licenses (GPL, AGPL,
   LGPL, MPL beyond what is already allowed) are not accepted.
4. **CodeQL passes.** No new high-severity findings in the Rust source
   or workflow YAML.
5. **Conventional commit prefix.** The PR's commits use
   [Conventional Commits](https://www.conventionalcommits.org/) —
   `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `refactor:`, `chore:`.
   `[skip release]` is reserved for the auto-tag CHANGELOG commit;
   do not use it manually.
6. **Verified signed commits.** The `main` branch protection rule
   requires all incoming commits to be signed. Sign your commits with
   either a [GPG](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits)
   or [SSH](https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification#ssh-commit-signature-verification)
   key configured on your GitHub account, or rebase through the GitHub
   web UI which signs automatically.
7. **License agreement.** Contributions are accepted under the
   [MIT License](LICENSE) only. By opening a PR you confirm that
   your contribution is yours to license under MIT.

### SHOULD (review expectations)

1. **Tests covering the change.** New features SHOULD ship with at
   least one integration test under `tests/` exercising the new
   behaviour end-to-end. Bug fixes SHOULD ship with a regression test
   that fails on `main` and passes with the fix.
2. **No coverage regression.** Codecov reports a delta on each PR.
   A small regression (a couple of percent on a non-critical path) is
   acceptable if justified in the PR description; large drops will be
   pushed back.
3. **`cargo fmt` applied.** `cargo fmt --check` should produce no
   diff. CI does not currently fail on this but it is the de-facto
   project style.
4. **`cargo clippy` clean.** Run
   `cargo clippy --all-targets -- -W clippy::all` and address findings
   in code you touched. Pre-existing warnings are not your problem.
5. **Edition 2021.** Stick to the `Cargo.toml` `edition` setting.
   Edition bumps are a separate, coordinated change.
6. **Minimal dependencies.** Prefer the standard library or an
   existing transitive dependency over pulling a new crate. Adding a
   new direct dependency SHOULD be justified in the PR description.
7. **Documentation in step with code.** User-facing behaviour changes
   SHOULD update the relevant docs:
   - `README.md` for top-level Quick Start or command summary.
   - `docs/src/commands.md` (and adjacent pages) for the mdBook site.
   The `CHANGELOG.md` is regenerated automatically from conventional
   commits; do not edit it by hand.
8. **Comments explain *why*, not *what*.** Default to no comments.
   Add one only when a non-obvious invariant, constraint, or workaround
   warrants it. Identifiers should carry the meaning.

## Coding conventions

- **Rust edition 2021.** The `Cargo.toml` `edition` field is the
  source of truth.
- **Tests.** Integration tests live under `tests/`. Unit tests live
  next to the code under `#[cfg(test)] mod tests {}`.
- **Comments.** See the SHOULD item above.
- **Dependencies.** See the MUST and SHOULD items above.

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
