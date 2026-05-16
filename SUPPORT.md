# Getting Support for rusta

This page tells you where to file what. Picking the right channel keeps
things moving and avoids cross-talk.

## Bug reports

If something does not work as documented, please open a **bug report**
on the issue tracker:

→ <https://github.com/pallewela/rusta/issues/new?labels=bug>

Useful information to include:

- `rusta --version`
- macOS version (`sw_vers -productVersion`) and chip (`uname -m`).
- The exact command you ran.
- The output you got vs. the output you expected.
- Re-run with `--verbose` and include the trailing output if relevant.

## Enhancement requests

For new features or behaviour changes, open a **feature request**:

→ <https://github.com/pallewela/rusta/issues/new?labels=enhancement>

Explain the use case first ("I want to X because Y"), then the proposed
shape of the feature. Concrete examples ("`rusta foo --bar` should
print Z") are easier to act on than abstract descriptions.

## Questions and discussion

For "how do I…?" questions, also use the
[issue tracker](https://github.com/pallewela/rusta/issues) — open an
issue with the `question` label. If GitHub Discussions are enabled on
the repo, they are equally welcome there.

## Security vulnerabilities

**Do not file security issues as public GitHub issues.** Use GitHub's
private vulnerability reporting flow as described in
[SECURITY.md](SECURITY.md).

## Response expectations

`rusta` is maintained on a best-effort basis by a single person.
Realistic expectations:

| Item | Target |
| --- | --- |
| Acknowledgement of a bug report | 7 days |
| Acknowledgement of a feature request | 14 days |
| Security advisory acknowledgement | 7 days |
| Patch for a confirmed regression | 30 days |

There are no paid support tiers and no SLAs. Pull requests are the
fastest way to get something fixed; see [CONTRIBUTING.md](CONTRIBUTING.md).
