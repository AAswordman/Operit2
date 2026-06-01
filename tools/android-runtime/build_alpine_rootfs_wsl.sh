#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../.." && pwd)"
asset_dir="$repo_dir/apps/flutter/app/android/app/src/main/assets/android-runtime"
cache_dir="$HOME/.cache/operit-android-runtime/alpine"
downloads_dir="$cache_dir/downloads"
work_dir="$cache_dir/work"
apk_static_dir="$cache_dir/apk-static"

mirror="https://dl-cdn.alpinelinux.org/alpine"
branch="latest-stable"
host_arch="x86_64"

packages=(
    bash
    python3
    nodejs
    npm
    uv
    pnpm
    ca-certificates
)

abis=(
    arm64-v8a
    armeabi-v7a
    x86_64
)

alpine_arch_for_abi() {
    case "$1" in
        arm64-v8a) echo aarch64 ;;
        armeabi-v7a) echo armv7 ;;
        x86_64) echo x86_64 ;;
        *) echo "Unsupported ABI: $1" >&2; exit 1 ;;
    esac
}

require_path() {
    local path="$1"
    test -e "$path" || {
        echo "Required path does not exist: $path" >&2
        exit 1
    }
}

download_checked() {
    local url="$1"
    local output="$2"
    local sha256="$3"

    mkdir -p "$(dirname "$output")"
    curl -fL -o "$output" "$url"
    printf '%s  %s\n' "$sha256" "$output" | sha256sum -c -
}

apk_index_record() {
    local repo="$1"
    local package_name="$2"
    local index_path="$downloads_dir/APKINDEX-$repo-$host_arch.tar.gz"

    mkdir -p "$downloads_dir"
    curl -fsSL -o "$index_path" "$mirror/$branch/$repo/$host_arch/APKINDEX.tar.gz"
    tar -xzO -f "$index_path" APKINDEX | awk -v package_name="$package_name" '
        BEGIN { RS = "" }
        found != 1 && $0 ~ ("(^|\n)P:" package_name "(\n|$)") {
            print
            found = 1
        }
    '
}

record_value() {
    local record="$1"
    local key="$2"
    awk -F: -v key="$key" '$1 == key && found != 1 { print substr($0, length(key) + 2); found = 1 }' <<< "$record"
}

install_apk_static() {
    local record
    local version
    local apk_name
    local apk_path

    record="$(apk_index_record main apk-tools-static)"
    version="$(record_value "$record" V)"
    test -n "$version"

    apk_name="apk-tools-static-$version.apk"
    apk_path="$downloads_dir/$apk_name"
    curl -fL -o "$apk_path" "$mirror/$branch/main/$host_arch/$apk_name"

    rm -rf "$apk_static_dir"
    mkdir -p "$apk_static_dir"
    tar -xzf "$apk_path" -C "$apk_static_dir" sbin/apk.static
    chmod 755 "$apk_static_dir/sbin/apk.static"
}

minirootfs_metadata() {
    local alpine_arch="$1"

    curl -fsSL "$mirror/$branch/releases/$alpine_arch/latest-releases.yaml" | awk '
        BEGIN { RS = "-\n"; FS = "\n" }
        found != 1 && $0 ~ /flavor: alpine-minirootfs/ {
            for (i = 1; i <= NF; i++) {
                line = $i
                sub(/^ +/, "", line)
                gsub(/"/, "", line)
                split(line, parts, ": ")
                if (parts[1] == "branch") release_branch = parts[2]
                if (parts[1] == "version") version = parts[2]
                if (parts[1] == "file") file = parts[2]
                if (parts[1] == "sha256") sha256 = parts[2]
            }
            print release_branch
            print version
            print file
            print sha256
            found = 1
        }
    '
}

write_rootfs_config() {
    local root_dir="$1"
    local release_branch="$2"

    cat > "$root_dir/etc/apk/repositories" <<EOF
$mirror/$release_branch/main
$mirror/$release_branch/community
EOF

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
    printf 'operit-android\n' > "$root_dir/etc/hostname"
}

build_rootfs_for_abi() {
    local abi="$1"
    local alpine_arch
    local metadata
    local release_branch
    local version
    local file
    local sha256
    local archive_path
    local root_dir
    local output_dir
    local output_path

    alpine_arch="$(alpine_arch_for_abi "$abi")"
    mapfile -t metadata < <(minirootfs_metadata "$alpine_arch")
    release_branch="${metadata[0]}"
    version="${metadata[1]}"
    file="${metadata[2]}"
    sha256="${metadata[3]}"

    test -n "$release_branch"
    test -n "$version"
    test -n "$file"
    test -n "$sha256"

    archive_path="$downloads_dir/$file"
    download_checked "$mirror/$release_branch/releases/$alpine_arch/$file" "$archive_path" "$sha256"

    root_dir="$work_dir/$abi/rootfs"
    output_dir="$asset_dir/$abi"
    output_path="$output_dir/rootfs.tar.gz.bin"

    rm -rf "$root_dir"
    mkdir -p "$root_dir" "$output_dir"
    tar -xzf "$archive_path" -C "$root_dir"
    write_rootfs_config "$root_dir" "$release_branch"

    "$apk_static_dir/sbin/apk.static" \
        --root "$root_dir" \
        --arch "$alpine_arch" \
        --usermode \
        --keys-dir "$root_dir/etc/apk/keys" \
        --repositories-file "$root_dir/etc/apk/repositories" \
        --no-cache \
        --no-scripts \
        --initdb \
        add "${packages[@]}"

    rm -rf "$root_dir/var/cache/apk"
    mkdir -p "$root_dir/var/cache/apk"
    printf '%s\n' "$version" > "$root_dir/etc/operit-alpine-version"
    printf '%s\n' "$abi" > "$root_dir/etc/operit-android-abi"

    tar --numeric-owner --sort=name --mtime='UTC 2026-01-01' -cf - -C "$root_dir" . | gzip -n > "$output_path"
    sha256sum "$output_path" > "$output_path.sha256"
}

mkdir -p "$downloads_dir" "$work_dir" "$asset_dir"
install_apk_static
require_path "$apk_static_dir/sbin/apk.static"

for abi in "${abis[@]}"; do
    build_rootfs_for_abi "$abi"
done
