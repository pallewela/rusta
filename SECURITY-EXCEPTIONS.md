# Security Tooling Exceptions

This document records deliberate exceptions to the security tools wired
into the repository (Scorecard / CodeQL / cargo-deny / etc.). Each entry
explains **what** is excepted, **why**, and **how** the resulting alert
is handled.

Keeping this list short and visible is itself a security practice: if
an exception ever stops being justified, it should show up in a
periodic review and get retired.

## SLSA reusable workflow pinned by tag

| Field | Value |
| --- | --- |
| Alert source | OpenSSF Scorecard, rule `Pinned-Dependencies` |
| Location | `.github/workflows/release.yml`, the `provenance` job's `uses:` line |
| Action involved | `slsa-framework/slsa-github-generator/.github/workflows/generator_generic_slsa3.yml@v2.1.0` |
| Status | **Intentional. Alert dismissed with reason `won't fix`.** |

### Why

The SLSA verifier (and the broader SLSA trust model) requires reusable
workflows to be referenced by tag, not by commit SHA. The verifier
matches the generator's released tag against an expected signed
identity. Pinning to a SHA breaks the chain that lets downstream
consumers verify the provenance with `slsa-verifier`.

The SLSA project documents this constraint:

- <https://github.com/slsa-framework/slsa-github-generator/blob/main/RELEASE.md>
- <https://github.com/slsa-framework/slsa-github-generator/issues/2998>

Scorecard's `Pinned-Dependencies` check has had an explicit exception
for `slsa-framework/slsa-github-generator` historically, but the
rule's SARIF output still raises a finding on this line for our
repo's deployment. The finding is dismissed in
**GitHub → Security → Code scanning** with reason `won't fix` and a
link back to this document.

### How to revisit this exception

If the SLSA generator publishes guidance saying SHA pinning is now
supported by `slsa-verifier`, or if Scorecard adds a structured
suppression mechanism, switch to SHA pinning and remove this entry.
