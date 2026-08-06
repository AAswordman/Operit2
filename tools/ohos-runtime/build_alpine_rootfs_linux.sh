#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../.." && pwd)"
output_dir="$repo_dir/apps/flutter/app/ohos/entry/src/main/resources/rawfile/ohos-vroot"
cache_dir="$HOME/.cache/operit-ohos-runtime/alpine"
downloads_dir="$cache_dir/downloads"
work_dir="$cache_dir/work"
apk_static_dir="$cache_dir/apk-static"

mirror="https://dl-cdn.alpinelinux.org/alpine"
release_branch="v3.22"
release_version="3.22.1"
alpine_arch="aarch64"
host_arch="x86_64"
archive_name="alpine-minirootfs-${release_version}-${alpine_arch}.tar.gz"
archive_url="$mirror/$release_branch/releases/$alpine_arch/$archive_name"

packages=(
    bash
    python3
    nodejs
    npm
    uv
    pnpm
    ca-certificates
)

## Verifies that a required path exists.
require_path() {
    local path="$1"
    test -e "$path" || {
        echo "Required path does not exist: $path" >&2
        exit 1
    }
}

## Downloads one URL with the network settings required by WSL environments.
download() {
    local url="$1"
    local output="$2"

    mkdir -p "$(dirname "$output")"
    curl --http1.1 --noproxy '*' --fail --location --output "$output" "$url"
}

## Downloads an Alpine APKINDEX record for one package.
apk_index_record() {
    local package_name="$1"
    local index_path="$downloads_dir/APKINDEX-main-$host_arch.tar.gz"

    download "$mirror/$release_branch/main/$host_arch/APKINDEX.tar.gz" "$index_path"
    tar -xzO -f "$index_path" APKINDEX | awk -v package_name="$package_name" '
        BEGIN { RS = "" }
        found != 1 && $0 ~ ("(^|\\n)P:" package_name "(\\n|$)") {
            print
            found = 1
        }
    '
}

## Extracts one field from an Alpine APKINDEX record.
record_value() {
    local record="$1"
    local key="$2"

    awk -F: -v key="$key" '$1 == key && found != 1 { print substr($0, length(key) + 2); found = 1 }' <<< "$record"
}

## Installs the host apk.static executable used to populate the aarch64 rootfs.
install_apk_static() {
    local record
    local version
    local apk_name
    local apk_path

    record="$(apk_index_record apk-tools-static)"
    version="$(record_value "$record" V)"
    test -n "$version"

    apk_name="apk-tools-static-$version.apk"
    apk_path="$downloads_dir/$apk_name"
    download "$mirror/$release_branch/main/$host_arch/$apk_name" "$apk_path"

    rm -rf "$apk_static_dir"
    mkdir -p "$apk_static_dir"
    tar -xzf "$apk_path" -C "$apk_static_dir" sbin/apk.static
    chmod 755 "$apk_static_dir/sbin/apk.static"
}

## Writes the package repositories and runtime directories into the Alpine rootfs.
write_rootfs_config() {
    local root_dir="$1"

    cat > "$root_dir/etc/apk/repositories" <<EOF
$mirror/$release_branch/main
$mirror/$release_branch/community
EOF

    mkdir -p \
        "$root_dir/dev" \
        "$root_dir/proc" \
        "$root_dir/sys" \
        "$root_dir/tmp" \
        "$root_dir/mnt/host-root" \
        "$root_dir/home/operit" \
        "$root_dir/etc/profile.d"
    chmod 1777 "$root_dir/tmp"

    cat > "$root_dir/etc/profile.d/operit.sh" <<'EOF'
export HOME=/home/operit
export LANG=C.UTF-8
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
EOF
    printf 'operit-ohos\n' > "$root_dir/etc/hostname"
}

## Builds the packaged Alpine rootfs with the toolchain required by tools and plugins.
build_rootfs() {
    local source_archive="$downloads_dir/$archive_name"
    local source_checksum="$downloads_dir/$archive_name.sha256"
    local root_dir="$work_dir/rootfs"
    local output_path="$output_dir/$archive_name"

    download "$archive_url" "$source_archive"
    download "$archive_url.sha256" "$source_checksum"
    (cd "$downloads_dir" && sha256sum --check "$archive_name.sha256")

    rm -rf "$root_dir"
    mkdir -p "$root_dir" "$output_dir"
    tar -xzf "$source_archive" -C "$root_dir"
    write_rootfs_config "$root_dir"

    env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
        "$apk_static_dir/sbin/apk.static" \
        --root "$root_dir" \
        --arch "$alpine_arch" \
        --keys-dir "$root_dir/etc/apk/keys" \
        --repositories-file "$root_dir/etc/apk/repositories" \
        --no-cache \
        --no-scripts \
        --initdb \
        add "${packages[@]}"

    rm -rf "$root_dir/var/cache/apk"
    mkdir -p "$root_dir/var/cache/apk"
    printf '%s\n' "$release_version" > "$root_dir/etc/operit-alpine-version"
    printf '%s\n' "$alpine_arch" > "$root_dir/etc/operit-ohos-arch"

    tar --numeric-owner --sort=name --mtime='UTC 2026-01-01' -cf - -C "$root_dir" . | gzip -n > "$output_path"
    sha256sum "$output_path" > "$output_path.sha256"
}

mkdir -p "$downloads_dir" "$work_dir" "$output_dir"
install_apk_static
require_path "$apk_static_dir/sbin/apk.static"
build_rootfs
