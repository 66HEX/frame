# Security Policy

## Supported versions

Security fixes are provided for the latest stable Frame release. Users should
upgrade to the newest published version before reporting a problem that may
already have been fixed.

## Report a vulnerability privately

Do not open a public issue for a suspected vulnerability, leaked credential,
malicious dependency, compromised release, or updater bypass.

Use [GitHub private vulnerability reporting](https://github.com/66HEX/frame/security/advisories/new)
when available. If that is not possible, email `hexthecoder@gmail.com` with:

- affected version and platform;
- reproduction steps or proof of concept;
- the expected security impact;
- relevant logs, hashes, URLs, or screenshots;
- whether the information has been shared with anyone else.

You should receive an acknowledgement within 48 hours. Status updates are
provided at least every seven days while the report remains open. Please allow
time for a coordinated fix and release before public disclosure.

## Release authenticity

Updater-managed macOS, Windows, and Linux tar releases use an Ed25519-signed
update manifest. The application verifies the manifest with an embedded public
key and checks downloaded assets against signed SHA-256 digests before
installation. GitHub releases also include `SHA256SUMS`, a CycloneDX SBOM, and
GitHub build-provenance attestations.

AppImage and Flatpak are intentionally outside Frame's native updater. Flatpak
updates are authenticated by its repository. AppImage update tools use the
AppImage/GitHub channel rather than Frame's signed manifest, so security-sensitive
users should verify `SHA256SUMS` and the GitHub attestation or use the managed
Linux tar release.

Release tags must be annotated, cryptographically signed, verified by GitHub,
and point to a commit already contained in `master`. Published release assets
are immutable: a failed or compromised release must be revoked and replaced by
a new version, never silently overwritten.

The macOS and Windows packages are distributed without Apple Developer ID or
Windows Authenticode certificates. macOS uses an ad-hoc signature required for
the application bundle to run. Package authenticity comes from the signed
update manifest, SHA-256 digests, and GitHub build-provenance attestations.

The private update key belongs only in a protected GitHub environment. It must
never be committed, copied into artifacts, added to logs, or exposed to
pull-request workflows.

## Dependency audit exceptions

`deny.toml` contains the complete, reasoned list of temporary RustSec exceptions
for transitive dependencies that cannot yet be upgraded independently of GPUI,
Wayland tooling, or Windows notification support. New advisories fail CI by
default. Every existing exception must be re-evaluated during dependency updates
and removed as soon as a compatible upstream release exists.
