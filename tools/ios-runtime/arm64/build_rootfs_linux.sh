#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != Linux ]]; then
    echo "Build the iOS ARM64 rootfs on Linux." >&2
    exit 1
fi
if [[ "$EUID" -ne 0 ]]; then
    echo "Run with sudo bash tools/ios-runtime/arm64/build_rootfs_linux.sh." >&2
    exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../../.." && pwd)"
output_dir="$repo_dir/apps/flutter/app/ios/Runner"
output_path="$output_dir/arm64-rootfs.tar.gz"
mirror="https://dl-cdn.alpinelinux.org/alpine"
release_branch="v3.23"
release_version="3.23.6"
alpine_arch="aarch64"
archive_name="alpine-minirootfs-${release_version}-${alpine_arch}.tar.gz"
archive_sha256="b17a57958e29735ff0e6e64254d958e65903687a70ca30cc430b33a965ad49d7"
source_date_epoch="${SOURCE_DATE_EPOCH:-1767225600}"

packages=(
    bash
    bash-completion
    "nodejs~24"
    npm
    pnpm
    python3
    py3-pip
    py3-virtualenv
    uv
    ca-certificates
    ca-certificates-bundle
    curl
    wget
    git
    openssh-client
    coreutils
    findutils
    diffutils
    grep
    sed
    gawk
    file
    tree
    less
    nano
    vim
    jq
    tar
    gzip
    bzip2
    xz
    zstd
    zip
    unzip
    util-linux
    procps
    iproute2
    iputils
    bind-tools
    openssl
    tzdata
    build-base
    linux-headers
    pkgconf
    cmake
    sqlite
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

for command in curl python3 tar gzip sha256sum chroot mount umount mountpoint mktemp grep install find; do
    command -v "$command" >/dev/null || {
        echo "Missing Linux build tool: $command" >&2
        exit 1
    }
done

qemu_path=""
case "$(uname -m)" in
    aarch64|arm64) ;;
    x86_64|amd64)
        qemu_path="$(command -v qemu-aarch64-static || true)"
        if [[ -z "$qemu_path" ]]; then
            echo "Install qemu-user-static and binfmt-support to build AArch64 packages on this host." >&2
            exit 1
        fi
        # Package post-install scripts must also be able to exec guest binaries.
        if ! grep -qs '^enabled' /proc/sys/fs/binfmt_misc/qemu-aarch64; then
            command -v update-binfmts >/dev/null || {
                echo "Missing update-binfmts; install binfmt-support." >&2
                exit 1
            }
            update-binfmts --enable qemu-aarch64
        fi
        if ! grep -qs '^enabled' /proc/sys/fs/binfmt_misc/qemu-aarch64 ||
            ! grep -qs '^flags:.*F' /proc/sys/fs/binfmt_misc/qemu-aarch64; then
            echo "Enable qemu-aarch64 binfmt with the F flag for guest package scripts in chroot." >&2
            exit 1
        fi
        ;;
    *) echo "Unsupported Linux build host: $(uname -m)" >&2; exit 1 ;;
esac

# Every invocation owns a fresh tree; existing runtime caches and roots are untouched.
work_dir="$(mktemp -d /tmp/operit-ios-arm64-rootfs.XXXXXX)"
root_dir="$work_dir/root"
output_tmp=""
mounted=()
cleanup() {
    local result=$?
    local unmount_failed=0
    local index
    trap - EXIT
    for ((index=${#mounted[@]}-1; index>=0; index--)); do
        if mountpoint -q "${mounted[$index]}" && ! umount "${mounted[$index]}"; then
            unmount_failed=1
        fi
    done
    if [[ "$unmount_failed" -eq 0 ]]; then
        rm -rf -- "$work_dir"
    else
        echo "Unmount failed; retained build tree at $work_dir." >&2
        result=1
    fi
    if [[ -n "$output_tmp" ]]; then rm -f -- "$output_tmp"; fi
    exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

curl --fail --location --retry 3 --retry-all-errors \
    "$mirror/$release_branch/releases/$alpine_arch/$archive_name" \
    --output "$work_dir/$archive_name"
printf '%s  %s\n' "$archive_sha256" "$work_dir/$archive_name" | sha256sum --check -
mkdir -p "$root_dir"
tar --numeric-owner -xzf "$work_dir/$archive_name" -C "$root_dir"

cat > "$root_dir/etc/apk/repositories" <<EOF
$mirror/$release_branch/main
$mirror/$release_branch/community
EOF
mkdir -p "$root_dir"/{dev,proc,sys,tmp,home/operit,host-root,mnt/host-root,sdcard,storage,etc/profile.d}
chmod 1777 "$root_dir/tmp"
cat /etc/resolv.conf > "$root_dir/etc/resolv.conf"
cat > "$root_dir/etc/profile.d/operit.sh" <<'EOF'
export HOME=/home/operit
export LANG=C.UTF-8
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
EOF
printf 'operit-ios-arm64\n' > "$root_dir/etc/hostname"
printf '%s\n' "$alpine_arch" > "$root_dir/etc/operit-ios-arm64-arch"
printf '%s\n' "$release_version" > "$root_dir/etc/operit-alpine-version"
printf '%s\n' "$mirror/$release_branch" > "$root_dir/etc/operit-alpine-repository"

mount --bind /dev "$root_dir/dev"
mounted+=("$root_dir/dev")
mount -t proc proc "$root_dir/proc"
mounted+=("$root_dir/proc")
guest=(chroot "$root_dir")
if [[ -n "$qemu_path" ]]; then
    install -m 755 "$qemu_path" "$root_dir/tmp/operit-qemu-aarch64-static"
    guest+=(/tmp/operit-qemu-aarch64-static)
fi
run_guest() {
    env -i HOME=/root LANG=C.UTF-8 \
        PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
        "${guest[@]}" "$@"
}

# apk verifies Alpine signatures, resolves dependencies and runs package triggers.
run_guest /sbin/apk --arch "$alpine_arch" --no-cache add "${packages[@]}"
run_guest /bin/busybox --install -s
run_guest /bin/sh /usr/sbin/update-ca-certificates
python3 "$repo_dir/tools/runtime/verify_alpine_node24.py" "$root_dir/lib/apk/db/installed"
install -m 644 "$repo_dir/tools/runtime/node24_smoke.cjs" "$root_dir/tmp/node24_smoke.cjs"
run_guest /usr/bin/node /tmp/node24_smoke.cjs

# Host mounts, DNS configuration and the build emulator do not belong in the image.
for ((index=${#mounted[@]}-1; index>=0; index--)); do
    umount "${mounted[$index]}"
done
mounted=()
rm -f -- "$root_dir/tmp/node24_smoke.cjs" "$root_dir/tmp/operit-qemu-aarch64-static"
# The runtime supplies the current host resolver as a virtual regular file.
# Build-host or hardcoded DNS servers may be unreachable on the iOS network.
rm -f -- "$root_dir/etc/resolv.conf"
mkdir -p "$root_dir/var/cache/apk"
find "$root_dir/var/cache/apk" -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +

mkdir -p "$output_dir"
output_tmp="$(mktemp "$output_dir/.arm64-rootfs.tar.gz.XXXXXX")"
tar --numeric-owner --owner=0 --group=0 --sort=name --mtime="@$source_date_epoch" \
    -cf - -C "$root_dir" . | gzip -n > "$output_tmp"
chmod 644 "$output_tmp"
mv -f -- "$output_tmp" "$output_path"
output_tmp=""
(cd "$output_dir" && sha256sum arm64-rootfs.tar.gz > arm64-rootfs.tar.gz.sha256)
printf 'Built %s (Alpine %s, %s)\n' "$output_path" "$release_version" "$alpine_arch"
