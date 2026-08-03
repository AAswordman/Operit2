#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../../.." && pwd)"
downloads_dir="$script_dir/downloads"
work_dir="$script_dir/build/rootfs"
rootfs_path="$repo_dir/apps/flutter/app/ios/Runner/ish-root.tar.gz"
root_archive="$downloads_dir/ish-root-appstore-apk.tar.gz"
apk_static_dir="$work_dir/apk-static"
root_dir="$work_dir/root"

root_url="https://github.com/ish-app/roots/releases/download/g00712ff0a54b2839c5aa1a8ed758003ca65357dc/appstore-apk.tar.gz"
root_sha256="776b16416894e5a8bec220b8d4726c46bb993289e8e48fdac54a519396e40a93"
repository_base="https://dl-cdn.alpinelinux.org/alpine/v3.19"
repository_arch="x86"
host_arch="x86_64"

packages=(
    bash
    python3
    py3-pip
    nodejs
    npm
    ca-certificates
)

# Downloads one artifact and verifies its SHA-256 digest.
download_checked() {
    local url="$1"
    local output="$2"
    local sha256="$3"

    mkdir -p "$(dirname "$output")"
    curl -fL --retry 3 --retry-all-errors -o "$output" "$url"
    printf '%s  %s\n' "$sha256" "$output" | sha256sum -c -
}

# Reads one APKINDEX entry for a host-native Alpine package.
apk_index_record() {
    local package_name="$1"
    local index_path="$downloads_dir/APKINDEX-main-$host_arch.tar.gz"

    curl -fL --retry 3 --retry-all-errors \
        -o "$index_path" \
        "$repository_base/main/$host_arch/APKINDEX.tar.gz"
    tar -xzO -f "$index_path" APKINDEX | awk -v package_name="$package_name" '
        BEGIN { RS = "" }
        found != 1 && $0 ~ ("(^|\\n)P:" package_name "(\\n|$)") {
            print
            found = 1
        }
    '
}

# Reads a named field from an APKINDEX package entry.
record_value() {
    local record="$1"
    local key="$2"

    awk -F: -v key="$key" '$1 == key && found != 1 { print substr($0, length(key) + 2); found = 1 }' <<< "$record"
}

# Prepares the host-native Alpine APK client used to populate the iSH rootfs.
install_apk_static() {
    local record
    local version
    local archive_name
    local archive_path

    record="$(apk_index_record apk-tools-static)"
    version="$(record_value "$record" V)"
    test -n "$version"

    archive_name="apk-tools-static-$version.apk"
    archive_path="$downloads_dir/$archive_name"
    curl -fL --retry 3 --retry-all-errors \
        -o "$archive_path" \
        "$repository_base/main/$host_arch/$archive_name"

    rm -rf "$apk_static_dir"
    mkdir -p "$apk_static_dir"
    tar -xzf "$archive_path" -C "$apk_static_dir" sbin/apk.static
    chmod 755 "$apk_static_dir/sbin/apk.static"
}

# Writes the shared Alpine user environment into the iSH rootfs.
write_rootfs_config() {
    mkdir -p \
        "$root_dir/dev" \
        "$root_dir/proc" \
        "$root_dir/sys" \
        "$root_dir/tmp" \
        "$root_dir/sdcard" \
        "$root_dir/storage" \
        "$root_dir/host-root" \
        "$root_dir/home/operit" \
        "$root_dir/etc/profile.d"
    chmod 1777 "$root_dir/tmp"

    cat > "$root_dir/etc/profile.d/operit.sh" <<'EOF'
export HOME=/home/operit
export LANG=C.UTF-8
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
EOF
    printf 'operit-ios-ish\n' > "$root_dir/etc/hostname"
    printf '%s\n' "$repository_arch" > "$root_dir/etc/operit-ios-ish-arch"
    printf '%s\n' "$repository_base" > "$root_dir/etc/operit-alpine-repository"
}

# Builds the iSH x86 Alpine rootfs with the Android-equivalent interpreter packages.
build_rootfs() {
    mkdir -p "$downloads_dir" "$work_dir" "$(dirname "$rootfs_path")"
    download_checked "$root_url" "$root_archive" "$root_sha256"

    rm -rf "$root_dir"
    mkdir -p "$root_dir"
    tar -xzf "$root_archive" -C "$root_dir"
    write_rootfs_config

    "$apk_static_dir/sbin/apk.static" \
        --root "$root_dir" \
        --arch "$repository_arch" \
        --usermode \
        --keys-dir "$root_dir/etc/apk/keys" \
        --repositories-file "$root_dir/etc/apk/repositories" \
        --no-cache \
        --no-scripts \
        add "${packages[@]}"

    rm -rf "$root_dir/var/cache/apk"
    mkdir -p "$root_dir/var/cache/apk"
    tar --numeric-owner --sort=name --mtime='UTC 2026-01-01' -cf - -C "$root_dir" . | gzip -n > "$rootfs_path"
}

install_apk_static
build_rootfs
