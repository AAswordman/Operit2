#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../.." && pwd)"
asset_dir="$repo_dir/apps/web_access/v86/runtime"
cache_dir="$HOME/.cache/operit-v86-runtime/buildroot-suite"
downloads_dir="$cache_dir/downloads"
builtin_downloads_dir="$downloads_dir/builtin"
python_runtime_dir="$cache_dir/python-3.12.11-i686"
work_dir="$cache_dir/work"

kernel_file="operit-runtime-bzimage.bin"
kernel_url="https://i.copy.sh/buildroot-bzimage.bin"
kernel_sha256="7befbaea31e249d9a518c4b95fa42b2a193d0e3de46250d617cbdeb866ee28b0"
node_archive="node-v20.19.0-linux-x86.tar.xz"
node_url="https://unofficial-builds.nodejs.org/download/release/v20.19.0/$node_archive"
node_sha256="6c0e9b2447569ec0e02a3cca93b71372085f86ce924fca0e74a359187f04a8bc"
python_archive="Python-3.12.11.tgz"
python_url="https://www.python.org/ftp/python/3.12.11/$python_archive"
python_sha256="7b8d59af8216044d2313de8120bfc2cc00a9bd2e542f15795e1d616c51faf3d6"

node_builtin_packages=(
    "lodash|lodash-4.17.21.tgz|https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz|6a087ac9e5702a0c9d60fbcd48696012646ec8df1491dea472b150e79fcaf804"
    "dayjs|dayjs-1.11.13.tgz|https://registry.npmjs.org/dayjs/-/dayjs-1.11.13.tgz|be0396c0f91583421b68f9a034227fb552eb87d93e59add775d533a65f70e47e"
    "zod|zod-3.24.2.tgz|https://registry.npmjs.org/zod/-/zod-3.24.2.tgz|f365a049bd1fcc3079e91d9cbcf968b7adce705662bfb3ca1ab3930c03b2ede3"
    "uuid|uuid-11.1.0.tgz|https://registry.npmjs.org/uuid/-/uuid-11.1.0.tgz|ec2013a661d5d449a872f259453bd59f1e0f30b60c57048949405b71914a1dc7"
)

python_builtin_packages=(
    "requests-2.32.3-py3-none-any.whl|https://files.pythonhosted.org/packages/f9/9b/335f9764261e915ed497fcdeb11df5dfd6f7bf257d4a6a2a686d80da4d54/requests-2.32.3-py3-none-any.whl|70761cfe03c773ceb22aa2f671b4757976145175cdfca038c02654d061d6dcc6"
    "urllib3-2.3.0-py3-none-any.whl|https://files.pythonhosted.org/packages/c8/19/4ec628951a74043532ca2cf5d97b7b14863931476d117c471e8e2b1eb39f/urllib3-2.3.0-py3-none-any.whl|1cee9ad369867bfdbbb48b7dd50374c0967a0bb7710050facf0dd6911440e3df"
    "idna-3.10-py3-none-any.whl|https://files.pythonhosted.org/packages/76/c6/c88e154df9c4e1a2a66ccf0005a88dfb2650c1dffb6f5ce603dfbd452ce3/idna-3.10-py3-none-any.whl|946d195a0d259cbba61165e88e65941f16e9b36ea6ddb97f00452bae8b1287d3"
    "certifi-2025.1.31-py3-none-any.whl|https://files.pythonhosted.org/packages/38/fc/bce832fd4fd99766c04d1ee0eead6b0ec6486fb100ae5e74c1d91292b982/certifi-2025.1.31-py3-none-any.whl|ca78db4565a652026a4db2bcdf68f2fb589ea80d0be70e03929ed730746b84fe"
    "charset_normalizer-3.4.1-py3-none-any.whl|https://files.pythonhosted.org/packages/0e/f6/65ecc6878a89bb1c23a086ea335ad4bf21a588990c3f535a227b9eea9108/charset_normalizer-3.4.1-py3-none-any.whl|d98b1668f06378c6dbefec3b92299716b931cd4e6061f3c875a71ced1780ab85"
    "rich-13.9.4-py3-none-any.whl|https://files.pythonhosted.org/packages/19/71/39c7c0d87f8d4e6c020a393182060eaefeeae6c01dab6a84ec346f2567df/rich-13.9.4-py3-none-any.whl|6049d5e6ec054bf2779ab3358186963bac2ea89175919d699e378b99738c2a90"
    "markdown_it_py-3.0.0-py3-none-any.whl|https://files.pythonhosted.org/packages/42/d7/1ec15b46af6af88f19b8e5ffea08fa375d433c998b8a7639e76935c14f1f/markdown_it_py-3.0.0-py3-none-any.whl|355216845c60bd96232cd8d8c40e8f9765cc86f46880e43a8fd22dc1a1a8cab1"
    "mdurl-0.1.2-py3-none-any.whl|https://files.pythonhosted.org/packages/b3/38/89ba8ad64ae25be8de66a6d463314cf1eb366222074cfda9ee839c56a4b4/mdurl-0.1.2-py3-none-any.whl|84008a41e51615a49fc9966191ff91509e3c40b939176e643fd50a5c2196b8f8"
    "pygments-2.18.0-py3-none-any.whl|https://files.pythonhosted.org/packages/f7/3f/01c8b82017c199075f8f788d0d906b9ffbbc5a47dc9918a945e13d5a2bda/pygments-2.18.0-py3-none-any.whl|b8e6aca0523f3ab76fee51799c488e38782ac06eafcf95e7ba832985c8e7b13a"
    "click-8.1.8-py3-none-any.whl|https://files.pythonhosted.org/packages/7e/d4/7ebdbd03970677812aac39c869717059dbb71a4cfc033ca6e5221787892c/click-8.1.8-py3-none-any.whl|63c132bbbed01578a06712a2d1f497bb62d9c1c0d329b7903a866228027263b2"
    "packaging-24.2-py3-none-any.whl|https://files.pythonhosted.org/packages/88/ef/eb23f262cca3c0c4eb7ab1933c3b1f03d021f2c48f54763065b6f0e321be/packaging-24.2-py3-none-any.whl|09abb1bccd265c01f4a3aa3f7a7db064b36514d2cba19a2f694fe6150451a759"
    "python_dateutil-2.9.0.post0-py2.py3-none-any.whl|https://files.pythonhosted.org/packages/ec/57/56b9bcc3c9c6a792fcbaf139543cee77261f3651ca9da0c93f5c1221264b/python_dateutil-2.9.0.post0-py2.py3-none-any.whl|a8b2bc7bffae282281c8140a97d3aa9c14da0b136dfe83f850eea9a5f7470427"
    "six-1.17.0-py2.py3-none-any.whl|https://files.pythonhosted.org/packages/b7/ce/149a00dd41f10bc29e5921b496af8b574d8413afcd5e30dfa0ed46c2cc5e/six-1.17.0-py2.py3-none-any.whl|4721f391ed90541fddacab5acf947aa0d3dc7d27b2e1e8eda2be8970586c3274"
)

## Returns whether the caller explicitly requested refreshed runtime downloads.
refresh_requested() {
    [[ "${OPERIT_RUNTIME_REFRESH:-0}" == "1" ]]
}

## Fails the build when one required Fedora host tool is unavailable.
require_command() {
    command -v "$1" >/dev/null
}

## Downloads and checksum-verifies one persistent runtime source archive.
download_verified() {
    local url="$1"
    local output_path="$2"
    local expected_sha256="$3"

    if refresh_requested || ! printf '%s  %s\n' "$expected_sha256" "$output_path" | sha256sum -c - >/dev/null 2>&1; then
        env -u https_proxy -u http_proxy -u all_proxy -u HTTPS_PROXY -u HTTP_PROXY -u ALL_PROXY \
            curl -fL -o "$output_path" "$url"
    fi
    printf '%s  %s\n' "$expected_sha256" "$output_path" | sha256sum -c - >&2
}

## Downloads the fixed offline package archives used by the browser runtime.
download_builtin_runtime_packages() {
    local specification
    local package_name
    local archive_name
    local package_url
    local package_sha256

    for specification in "${node_builtin_packages[@]}"; do
        IFS='|' read -r package_name archive_name package_url package_sha256 <<< "$specification"
        download_verified "$package_url" "$builtin_downloads_dir/$archive_name" "$package_sha256"
    done
    for specification in "${python_builtin_packages[@]}"; do
        IFS='|' read -r archive_name package_url package_sha256 <<< "$specification"
        download_verified "$package_url" "$builtin_downloads_dir/$archive_name" "$package_sha256"
    done
}

## Builds a reusable i686 CPython installation with Fedora's native compiler.
prepare_python_runtime() {
    local source_archive="$downloads_dir/$python_archive"
    local source_dir="$work_dir/Python-3.12.11"
    local install_root="$python_runtime_dir/usr/local/operit-python"

    if ! refresh_requested && [[ -x "$install_root/bin/python3.12" ]]; then
        return
    fi

    rm -rf "$python_runtime_dir" "$source_dir"
    mkdir -p "$python_runtime_dir" "$source_dir"
    tar -xzf "$source_archive" -C "$work_dir"
    pushd "$source_dir" >/dev/null
    CC="gcc -m32" \
    CXX="g++ -m32" \
    CFLAGS="-O2 -pipe -m32" \
    LDFLAGS="-m32" \
    ./configure \
        --prefix=/usr/local/operit-python \
        --without-ensurepip \
        --disable-test-modules
    make -j"$(nproc)"
    make DESTDIR="$python_runtime_dir" altinstall
    popd >/dev/null

    rm -rf \
        "$install_root/include" \
        "$install_root/share" \
        "$install_root/lib/python3.12/test" \
        "$install_root/lib/python3.12/ensurepip"
    find "$install_root" -type d -name __pycache__ -prune -exec rm -rf {} +
    test -x "$install_root/bin/python3.12"
}

## Packages CPython's pure-Python standard library in its native zip import path.
package_python_standard_library() {
    local install_root="$python_runtime_dir/usr/local/operit-python"
    local standard_library="$install_root/lib/python3.12"
    local archive="$install_root/lib/python312.zip"

    if ! refresh_requested && [[ -s "$archive" ]]; then
        return
    fi

    rm -f "$archive"
    pushd "$standard_library" >/dev/null
    zip -q -9 -r "$archive" . -i '*.py'
    popd >/dev/null
    find "$standard_library" -type f -name '*.py' -delete
    test -s "$archive"
}

## Copies one ELF and every loader-resolved dependency into the initramfs tree.
copy_runtime_dependencies() {
    local source_path="$1"
    local initramfs_root="$2"
    local dependency

    while IFS= read -r dependency; do
        if [[ "$dependency" == "$source_path" ]]; then
            continue
        fi
        test -f "$dependency"
        install -Dm0755 "$dependency" "$initramfs_root$dependency"
    done < <(lddtree -l "$source_path")
}

## Expands fixed JavaScript packages into Node's offline global module directory.
stage_builtin_node_packages() {
    local initramfs_root="$1"
    local specification
    local package_name
    local archive_name
    local package_url
    local package_sha256

    for specification in "${node_builtin_packages[@]}"; do
        IFS='|' read -r package_name archive_name package_url package_sha256 <<< "$specification"
        mkdir -p "$initramfs_root/usr/local/lib/node_modules/$package_name"
        tar -xzf "$builtin_downloads_dir/$archive_name" \
            -C "$initramfs_root/usr/local/lib/node_modules/$package_name" \
            --strip-components=1
    done
}

## Expands fixed Python wheels into CPython's offline site-packages directory.
stage_builtin_python_packages() {
    local initramfs_root="$1"
    local site_packages="$initramfs_root/usr/local/operit-python/lib/python3.12/site-packages"
    local specification
    local archive_name
    local package_url
    local package_sha256

    mkdir -p "$site_packages"
    for specification in "${python_builtin_packages[@]}"; do
        IFS='|' read -r archive_name package_url package_sha256 <<< "$specification"
        unzip -q "$builtin_downloads_dir/$archive_name" -d "$site_packages"
    done
    find "$site_packages" -type d -name __pycache__ -prune -exec rm -rf {} +
}

## Copies the Node and Python runtime files into the dedicated V86 initramfs tree.
stage_runtime_programs() {
    local initramfs_root="$1"
    local node_root="$work_dir/node"
    local python_root="$python_runtime_dir/usr/local/operit-python"
    local extension

    rm -rf "$node_root"
    mkdir -p "$node_root"
    tar -xJf "$downloads_dir/$node_archive" -C "$node_root" --strip-components=1
    install -Dm0755 "$node_root/bin/node" "$initramfs_root/usr/local/bin/node"
    strip --strip-unneeded "$initramfs_root/usr/local/bin/node"
    stage_builtin_node_packages "$initramfs_root"
    copy_runtime_dependencies "$node_root/bin/node" "$initramfs_root"

    mkdir -p "$initramfs_root/usr/bin"
    mkdir -p "$initramfs_root/usr/local/operit-python"
    cp -a "$python_root/." "$initramfs_root/usr/local/operit-python/"
    rm -rf "$initramfs_root/usr/local/operit-python/lib/python3.12/config-3.12-i386-linux-gnu"
    find "$initramfs_root/usr/local/operit-python" -type f -name '*.a' -delete
    find "$initramfs_root/usr/local/operit-python" -type d -name __pycache__ -prune -exec rm -rf {} +
    strip --strip-unneeded "$initramfs_root/usr/local/operit-python/bin/python3.12"
    find "$initramfs_root/usr/local/operit-python/lib/python3.12/lib-dynload" -type f -name '*.so' -exec strip --strip-unneeded {} +
    ln -s ../local/bin/node "$initramfs_root/usr/bin/node"
    ln -s ../local/operit-python/bin/python3.12 "$initramfs_root/usr/bin/python"
    ln -s ../local/operit-python/bin/python3.12 "$initramfs_root/usr/bin/python3"
    copy_runtime_dependencies "$python_root/bin/python3.12" "$initramfs_root"
    while IFS= read -r extension; do
        copy_runtime_dependencies "$extension" "$initramfs_root"
    done < <(find "$python_root/lib/python3.12/lib-dynload" -type f -name '*.so' | LC_ALL=C sort)
    find "$initramfs_root/lib" -type f -exec strip --strip-unneeded {} +
    ln -s ../operit-python/bin/python3.12 "$initramfs_root/usr/local/bin/python3"
    stage_builtin_python_packages "$initramfs_root"
}

## Builds and stages the serial-to-PTY relay used by manual Linux terminals.
stage_terminal_relay() {
    local initramfs_root="$1"
    local relay_path="$work_dir/operit-runtime-terminal"

    gcc -m32 -O2 -pipe "$script_dir/operit-runtime-terminal.c" -o "$relay_path" -lutil
    install -Dm0755 "$relay_path" "$initramfs_root/usr/local/bin/operit-runtime-terminal"
    copy_runtime_dependencies "$relay_path" "$initramfs_root"
}

## Writes the init program that selects either an interactive shell or the managed agent.
write_init_programs() {
    local initramfs_root="$1"

    install -Dm0755 "$script_dir/operit-runtime-agent.py" "$initramfs_root/usr/local/bin/operit-runtime-agent"
    cat > "$initramfs_root/usr/local/bin/operit-runtime-init" <<'EOF'
#!/bin/sh
exec /usr/local/bin/python3 /usr/local/bin/operit-runtime-agent
EOF
    chmod 0755 "$initramfs_root/usr/local/bin/operit-runtime-init"
    cat > "$initramfs_root/init" <<'EOF'
#!/bin/sh
mount -t devtmpfs devtmpfs /dev
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mkdir -p /dev/pts
mount -t devpts devpts /dev/pts
export HOME=/workspace
export PATH=/usr/local/bin:/usr/bin:/bin:/sbin
export TERM=xterm-256color
export PYTHONHOME=/usr/local/operit-python
export NODE_PATH=/usr/local/lib/node_modules
mkdir -p /workspace
for argument in $(cat /proc/cmdline); do
    case "$argument" in
        operit.rows=*)
            terminal_rows="${argument#operit.rows=}"
            ;;
        operit.cols=*)
            terminal_cols="${argument#operit.cols=}"
            ;;
        operit.mode=agent)
            exec /usr/local/bin/operit-runtime-init </dev/ttyS0 >/dev/ttyS0 2>&1
            ;;
    esac
done
exec /usr/local/bin/operit-runtime-terminal "$terminal_rows" "$terminal_cols" </dev/ttyS0 >/dev/ttyS0 2>&1
EOF
    chmod 0755 "$initramfs_root/init"
}

## Builds a gzip-compressed newc initramfs for the 32-bit V86 Buildroot kernel.
build_initramfs() {
    local initramfs_root="$work_dir/initramfs"
    local output_path="$1"

    rm -rf "$initramfs_root"
    mkdir -p "$initramfs_root"
    stage_runtime_programs "$initramfs_root"
    stage_terminal_relay "$initramfs_root"
    write_init_programs "$initramfs_root"
    (
        cd "$initramfs_root"
        find . -print0 | LC_ALL=C sort -z | cpio --null --owner=0:0 -o -H newc | gzip -9 > "$output_path"
    )
    test -s "$output_path"
}

## Writes the manifest that binds the Web launcher to the generated 32-bit suite.
write_manifest() {
    local kernel_path="$1"
    local initrd_path="$2"
    local manifest_path="$asset_dir/operit-runtime-manifest.json"

    cat > "$manifest_path" <<EOF
{
  "formatVersion": 2,
  "architecture": "i686",
  "kernel": {"file": "$(basename "$kernel_path")", "sha256": "$(sha256sum "$kernel_path" | awk '{print $1}')", "bytes": $(stat -c%s "$kernel_path")},
  "initrd": {"file": "$(basename "$initrd_path")", "sha256": "$(sha256sum "$initrd_path" | awk '{print $1}')", "bytes": $(stat -c%s "$initrd_path")},
  "programs": ["busybox", "node", "python", "python3"],
  "agent": "/usr/local/bin/operit-runtime-agent"
}
EOF
}

## Builds the 32-bit Buildroot V86 runtime suite on Fedora WSL.
main() {
    local kernel_path="$asset_dir/$kernel_file"
    local initrd_path="$asset_dir/operit-runtime-initrd.cpio.gz"
    local initrd_build_path="$work_dir/operit-runtime-initrd.cpio.gz"

    require_command cpio
    require_command curl
    require_command find
    require_command gcc
    require_command g++
    require_command gzip
    require_command lddtree
    require_command make
    require_command sha256sum
    require_command strip
    require_command tar
    require_command unzip
    require_command zip
    mkdir -p "$asset_dir" "$downloads_dir" "$builtin_downloads_dir" "$work_dir"
    download_verified "$kernel_url" "$downloads_dir/buildroot-bzimage.bin" "$kernel_sha256"
    download_verified "$node_url" "$downloads_dir/$node_archive" "$node_sha256"
    download_verified "$python_url" "$downloads_dir/$python_archive" "$python_sha256"
    download_builtin_runtime_packages
    prepare_python_runtime
    package_python_standard_library
    build_initramfs "$initrd_build_path"
    install -Dm0644 "$downloads_dir/buildroot-bzimage.bin" "$kernel_path"
    install -Dm0644 "$initrd_build_path" "$initrd_path"
    rm -f "$asset_dir/operit-runtime-rootfs.ext2"
    write_manifest "$kernel_path" "$initrd_path"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
