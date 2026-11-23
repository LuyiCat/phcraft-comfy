# Release security

This fork releases from `main` via annotated `vX.Y.Z` tags and GitHub Releases; the authoritative process is the fork's [`agent.md`](https://github.com/huairuiLove/phcraft-comfy/blob/main/agent.md) (§10/§10.1), summarized in [`docs/releasing.md`](https://github.com/huairuiLove/phcraft-comfy/blob/main/docs/releasing.md). The long-lived `release` branch is no longer used as a release line: the release workflow now triggers on `v*` tag pushes (or a controlled manual run with a version override) and builds the Android APK alongside the desktop packages.

The elements below describe the current release workflow.

## Implemented workflow elements

- platform-specific signing/notarization steps exist for macOS and Windows;
- desktop signing secrets are scoped through the GitHub `release` environment of the shared release workflow; the Android signing secrets are repository-level;
- the Azure Trusted Signing client that Windows signing loads is pinned to a version and SHA-256, and refused on a mismatch;
- release artifacts include `SHA256SUMS.txt`;
- the final release job receives `contents: write`, while build jobs use narrower access;
- artifacts remain in a draft release until a maintainer publishes them.

Signing is conditional. The workflow can produce unsigned artifacts when signing material is unavailable and emits warnings. Consumers and maintainers must verify the job summary and artifact signature rather than assuming every release is signed.

## Future hardening

- generate a machine-readable SBOM for each release;
- publish provenance/attestations tied to the source commit and workflow;
- make signature state explicit in release metadata;
- verify checksums, signatures, notarization, and installer behavior before publication;
- review action pinning and release-tool provenance;
- document key rotation and compromise response.

Building, signing, drafting, publishing, and deploying are separate states. A successful build must not be reported as a published release.
