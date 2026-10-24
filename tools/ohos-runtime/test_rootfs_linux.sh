#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../.." && pwd)"
archive="$repo_dir/apps/flutter/app/ohos/entry/src/main/resources/rawfile/ohos-vroot/alpine-minirootfs-3.23.6-aarch64.tar.gz"

if [[ "$EUID" -ne 0 ]]; then
    echo "Run the ARM64 rootfs integration checks as root with qemu-user installed." >&2
    exit 1
fi

root="$(mktemp -d /tmp/operit-ohos-rootfs-test.XXXXXXXX)"

## Removes only this invocation's generated test root.
cleanup() {
    local target
    target="$(realpath "$root")"
    case "$target" in
        /tmp/operit-ohos-rootfs-test.*) rm -rf -- "$target" ;;
        *) echo "Unexpected test root: $target" >&2; exit 1 ;;
    esac
}
trap cleanup EXIT

## Installs the exact bundled packages without accessing network repositories.
install_packages() {
    local packages
    mapfile -t packages < "$root/etc/operit/packages.list"
    chroot "$root" /sbin/apk --no-network --allow-untrusted --no-scripts \
        --repositories-file /etc/operit/repositories --repository /etc/operit/packages \
        add --upgrade "${packages[@]}"
    chroot "$root" /bin/busybox --install -s
}

tar -xzf "$archive" -C "$root"
install_packages
chroot "$root" /usr/bin/python3 --version
chroot "$root" /usr/bin/node --version
chroot "$root" /usr/bin/uv --version
chroot "$root" /usr/bin/pnpm --version
chroot "$root" /usr/bin/tesseract --list-langs
chroot "$root" /usr/bin/pdftoppm -v

# An update must retain both user files and user package database entries.
chroot "$root" /sbin/apk --no-network --repositories-file /etc/operit/repositories \
    add --force-non-repository --virtual operit-user-test
printf 'user-terminal-data\n' > "$root/root/user-data.txt"
install_packages
chroot "$root" /sbin/apk info --installed operit-user-test
test "$(cat "$root/root/user-data.txt")" = user-terminal-data

# Exercise the actual guest PDF script against a two-page document and OCR its result.
python3 - "$repo_dir" "$root" <<'PY'
import pathlib, re, sys
repo, root = map(pathlib.Path, sys.argv[1:])
objects = [
    b'<< /Type /Catalog /Pages 2 0 R >>',
    b'<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>',
    b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 6 0 R >>',
    b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 7 0 R >>',
    b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
]
for text in [b'FIRST PAGE OPERIT 2026', b'SECOND PAGE OPERIT 2026']:
    stream = b'BT /F1 28 Tf 40 600 Td (' + text + b') Tj ET'
    objects.append(b'<< /Length ' + str(len(stream)).encode() + b' >>\nstream\n' + stream + b'\nendstream')
result = bytearray(b'%PDF-1.4\n')
offsets = [0]
for index, obj in enumerate(objects, 1):
    offsets.append(len(result))
    result.extend(str(index).encode() + b' 0 obj\n' + obj + b'\nendobj\n')
xref = len(result)
result.extend(b'xref\n0 8\n0000000000 65535 f \n')
for offset in offsets[1:]:
    result.extend(f'{offset:010d} 00000 n \n'.encode())
result.extend(b'trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n' + str(xref).encode() + b'\n%%EOF\n')
(root / 'tmp/source.pdf').write_bytes(result)
source = (repo / 'hosts/ohos/src/documents.rs').read_text()
script = re.search(r'const PDF_SCRIPT: &str = r#"(.*?)"#;', source, re.S).group(1)
(root / 'tmp/render.py').write_text(script)
(root / 'tmp/rendered').mkdir()
PY
chroot "$root" /usr/bin/python3 /tmp/render.py /tmp/source.pdf /tmp/rendered '[1,0]' 2
chroot "$root" /usr/bin/tesseract /tmp/rendered/page-0.png /tmp/result -l eng --psm 3
python3 - "$root" <<'PY'
import json, pathlib, struct, sys
root = pathlib.Path(sys.argv[1])
assert json.loads((root / 'tmp/rendered/pages.json').read_text()) == ['page-0.png', 'page-1.png']
png = (root / 'tmp/rendered/page-0.png').read_bytes()
assert struct.unpack('>II', png[16:24]) == (1224, 1584)
assert 'SECOND PAGE OPERIT 2026' in (root / 'tmp/result.txt').read_text()
assert (root / 'etc/ssl/cert.pem').read_bytes()
print('ARM64 offline installation, package preservation, runtimes, PDF order/scale, and OCR passed.')
PY
