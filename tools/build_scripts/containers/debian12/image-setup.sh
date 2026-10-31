#!/usr/bin/env bash
# Provisioning for the Debian 12 CLI build image. Executed as root in a
# scratch container by build_cli_linux_debian12.py, then committed as the
# operit2-build:debian12 image. Driving the setup through `podman run` +
# `podman commit` instead of `podman build` avoids buildah's kernel-overlayfs
# build-context mounts, which kernels without the overlay module cannot
# service.
set -euo pipefail

node_major="${NODE_MAJOR:-24}"
# Scheme-less Debian mirror base such as mirrors.ustc.edu.cn (empty keeps
# deb.debian.org). Minimal base images ship without ca-certificates, so the
# first apt round stays on http and installs them before sources move to
# https.
sources=/etc/apt/sources.list.d/debian.sources
[ -f "$sources" ] || sources=/etc/apt/sources.list

if [ -n "${DEBIAN_MIRROR:-}" ]; then
  sed -i "s|http://deb.debian.org/|http://${DEBIAN_MIRROR}/|g" "$sources"
fi

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends ca-certificates curl

if [ -n "${DEBIAN_MIRROR:-}" ]; then
  sed -i "s|http://${DEBIAN_MIRROR}/|https://${DEBIAN_MIRROR}/|g" "$sources"
fi
apt-get update

# gcc-x86-64-linux-gnu is the native compiler on amd64 bases and the cross
# compiler on arm64 bases, so both cross targets stay buildable. cmake is
# required by the operit-local-models build script even without the
# native-llm feature. The cross libc package provides the target-side
# headers and libraries for the non-native architecture.
cross_libc_dev=libc6-dev-arm64-cross
if [ "$(dpkg --print-architecture)" != "amd64" ]; then
  cross_libc_dev=libc6-dev-amd64-cross
fi
apt-get install -y --no-install-recommends \
  build-essential \
  cmake \
  "$cross_libc_dev" \
  g++-aarch64-linux-gnu \
  g++-x86-64-linux-gnu \
  gcc-aarch64-linux-gnu \
  gcc-x86-64-linux-gnu \
  git \
  gnupg \
  pkg-config \
  python3 \
  xz-utils
curl -fsSL "https://deb.nodesource.com/setup_${node_major}.x" | bash -
apt-get install -y --no-install-recommends nodejs
rm -rf /var/lib/apt/lists/*
