# Security Policy

## Supported Versions

We currently only provide security updates for the latest version of Frame. Please ensure you are using the most recent release.

## Update Authenticity

Native Frame releases use a signed update manifest. The application verifies the
manifest with an embedded Ed25519 public key and checks each downloaded asset
against the SHA-256 value recorded in that signed manifest before installation.

The private update signing key must stay in GitHub Actions secrets or an
equivalent restricted release environment. Do not commit it to the repository or
ship it inside an application bundle.

## Reporting a Vulnerability

We take the security of Frame seriously. If you believe you've found a security vulnerability, please do NOT open a public issue.

Instead, please report it privately by emailing hexthecoder@gmail.com.

We will acknowledge your report within 48 hours and provide a timeline for a fix. Please provide as much detail as possible, including steps to reproduce the vulnerability.
