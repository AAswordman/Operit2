#!/usr/bin/env bash
set -euo pipefail

# Root ownership and executable modes must be preserved in the guest image.
if [[ "$EUID" -ne 0 ]]; then
    echo "Build the Alpine image as root (wsl -u root)." >&2
    exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../.." && pwd)"
output_dir="$repo_dir/apps/flutter/app/ohos/entry/src/main/resources/rawfile/ohos-vroot"
cache_dir="$HOME/.cache/operit-ohos-runtime/alpine"

## Removes only generated paths contained in this build cache.
reset_generated_directory() {
    local target
    target="$(realpath -m "$1")"
    case "$target" in
        "$cache_dir"/*) rm -rf -- "$target" ;;
        *) echo "Refusing to reset a path outside the runtime cache: $target" >&2; exit 1 ;;
    esac
    mkdir -p "$target"
}
downloads_dir="$cache_dir/downloads"
work_dir="$cache_dir/work"
apk_static_dir="$cache_dir/apk-static"

mirror="https://dl-cdn.alpinelinux.org/alpine"
release_branch="v3.23"
release_version="3.23.6"
alpine_arch="aarch64"
host_arch="x86_64"
archive_name="alpine-minirootfs-${release_version}-${alpine_arch}.tar.gz"
archive_url="$mirror/$release_branch/releases/$alpine_arch/$archive_name"

packages=(
    bash
    python3
    "nodejs~24"
    npm
    uv
    pnpm
    ca-certificates
    curl
    git
    openssh-client
    tesseract-ocr
    tesseract-ocr-data-eng
    tesseract-ocr-data-chi_sim
    tesseract-ocr-data-chi_tra
    tesseract-ocr-data-jpn
    tesseract-ocr-data-kor
    poppler-utils
    font-noto
    font-noto-cjk
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

    reset_generated_directory "$apk_static_dir"
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

    reset_generated_directory "$root_dir"
    mkdir -p "$output_dir"
    tar -xzf "$source_archive" -C "$root_dir"
    write_rootfs_config "$root_dir"

    # Package updates are applied by apk so installed user packages keep their database entries.
    local repository="$root_dir/etc/operit/packages/$alpine_arch"
    mkdir -p "$repository"
    ln -s "$alpine_arch" "$root_dir/etc/operit/packages/noarch"
    env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
        "$apk_static_dir/sbin/apk.static" --root "$root_dir" --arch "$alpine_arch" \
        --repositories-file "$root_dir/etc/apk/repositories" --no-cache fetch --recursive \
        --output "$repository" "${packages[@]}"
    "$apk_static_dir/sbin/apk.static" --keys-dir "$root_dir/etc/apk/keys" --arch "$alpine_arch" index \
        --output "$repository/APKINDEX.tar.gz" "$repository"/*.apk
    tar -xzOf "$repository/APKINDEX.tar.gz" APKINDEX > "$work_dir/APKINDEX"
    python3 "$repo_dir/tools/runtime/verify_alpine_node24.py" "$work_dir/APKINDEX"
    # The host installer accepts package names; the verified repository pins their versions.
    printf '%s\n' "${packages[@]%%~*}" > "$root_dir/etc/operit/packages.list"
    : > "$root_dir/etc/operit/repositories"

    reset_generated_directory "$root_dir/var/cache/apk"
    printf '%s\n' "$release_version" > "$root_dir/etc/operit-alpine-version"
    printf '%s\n' "$alpine_arch" > "$root_dir/etc/operit-ohos-arch"

    tar --numeric-owner --sort=name --mtime='UTC 2026-01-01' -cf - -C "$root_dir" . | gzip -n > "$output_path"
    (cd "$output_dir" && sha256sum "$archive_name" > "$archive_name.sha256")
}

mkdir -p "$downloads_dir" "$work_dir" "$output_dir"
install_apk_static
require_path "$apk_static_dir/sbin/apk.static"
build_rootfs
