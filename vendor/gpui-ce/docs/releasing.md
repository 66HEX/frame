# Releasing `gpui-ce`

The committed `gpui-ce` version stays at `0.2.2`. Do not change it for alpha
publishes, commit an alpha suffix, or create an alpha Git tag.

## Alpha snapshots

After CI succeeds, only the latest commit on `main` is published as a crates.io
development snapshot named `1.0.0-alpha.N`: the workflow derives `1` from the
committed major `0`, and discovers `N` from crates.io. Older queued CI runs
self-discard when main has advanced. It neither reads nor creates Git tags. It
temporarily substitutes the alpha version while packaging; the repository and
its Git history continue to record `0.2.2`.

Cargo excludes prereleases from ordinary dependency resolution. Consequently,
`cargo add gpui-ce` continues to select the latest stable crate. Consumers who
want a snapshot must opt in with an explicit alpha version, for example
`cargo add gpui-ce@1.0.0-alpha.3`.

Stable tagged releases continue to use the existing tag workflow and do not
change the alpha publisher's committed-version record.

## SemVer checks

SemVer CI compares the PR head directly with the PR base commit. That isolates
the API changes introduced by the PR and does not re-report incompatible API
changes that were already on `main`.
