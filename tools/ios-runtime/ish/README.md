# Historical iSH caches

The iSH x86 backend has been removed from the iOS app, terminal routing and CI.
Its source preparation, build scripts and custom patches are no longer build
inputs. This directory may still contain ignored download/build caches from an
earlier checkout; existing user roots and ignored archives are preserved.

Use [`../arm64/`](../arm64/README.md) to build the independent ARM64 runtime and
Alpine AArch64 development environment. Do not reuse an x86 root as an ARM64
root: executables, libraries and the package database must match the architecture.
