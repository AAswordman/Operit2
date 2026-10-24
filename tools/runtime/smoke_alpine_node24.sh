#!/usr/bin/env bash
# Executes Node from the packaged rootfs, including cross-architecture ARM64 roots.
set -euo pipefail
archive="$(realpath "$1")"
architecture="$2"
package_layout="$3"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(mktemp -d)"
trap 'sudo umount "$root/dev" 2>/dev/null || true; sudo rm -rf "$root"' EXIT
sudo tar -xzf "$archive" -C "$root"
sudo mkdir -p "$root/tmp" "$root/dev"
sudo mount --bind /dev "$root/dev"
sudo cp "$script_dir/node24_smoke.cjs" "$root/tmp/node24_smoke.cjs"
guest=(sudo chroot "$root")
case "$architecture" in
    x86) ;;
    aarch64)
        sudo cp /usr/bin/qemu-aarch64-static "$root/tmp/qemu-aarch64-static"
        guest+=(/tmp/qemu-aarch64-static)
        ;;
    *) echo "Unsupported runtime architecture: $architecture" >&2; exit 1 ;;
esac
case "$package_layout" in
    preinstalled) ;;
    offline)
        mapfile -t packages < "$root/etc/operit/packages.list"
        "${guest[@]}" /sbin/apk --no-network --allow-untrusted --no-scripts \
            --repositories-file /etc/operit/repositories --repository /etc/operit/packages \
            add --upgrade "${packages[@]}"
        "${guest[@]}" /bin/busybox --install -s
        ;;
    *) echo "Unsupported runtime package layout: $package_layout" >&2; exit 1 ;;
esac
python3 "$script_dir/verify_alpine_node24.py" "$root/lib/apk/db/installed"
"${guest[@]}" /usr/bin/node /tmp/node24_smoke.cjs
