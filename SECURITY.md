# Security Policy

## Supported Versions

Only the **latest** release of `rusta` is supported with security updates.
Older versions remain available on the
[releases page](https://github.com/pallewela/rusta/releases) but will not
receive patches.

| Version | Supported          |
| ------- | ------------------ |
| Latest  | :white_check_mark: |
| Older   | :x:                |

## Reporting a Vulnerability

**Do not file a public GitHub issue for security vulnerabilities.**

Please use GitHub's private vulnerability reporting:

1. Open https://github.com/pallewela/rusta/security/advisories/new
2. Describe the issue with as much detail as possible:
   - affected version(s) (`rusta --version`)
   - reproduction steps
   - impact and any proposed mitigation

You should receive an acknowledgement within **7 days**. We aim to ship a
fix or a status update within **30 days** of confirmation.

## Scope

In scope:

- The `rusta` binary and any code in this repository.
- The release artifacts published to GitHub Releases and crates.io
  (`rusta-cli`).
- The Homebrew formula at
  [`pallewela/homebrew-tap`](https://github.com/pallewela/homebrew-tap).

Out of scope:

- Vulnerabilities in [Tart](https://tart.run/) itself — report those to
  the Tart project.
- Vulnerabilities in the guest Ubuntu images we clone — report those to
  Canonical.
- Vulnerabilities in third-party crates we depend on — report those to
  the crate maintainers and the [RustSec Advisory
  Database](https://rustsec.org/contributing.html).

## Disclosure

Once a fix is available we will:

1. Publish a patch release.
2. File a [GitHub Security Advisory](https://github.com/pallewela/rusta/security/advisories).
3. Credit the reporter (unless anonymity is requested).
