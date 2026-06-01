#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd "$script_dir/../.." && pwd)"
runtime_dir="$repo_dir/tools/android-runtime"
source_dir="$runtime_dir/sources"
build_dir="$HOME/.cache/operit-android-runtime/build/assistance2"
asset_dir="$repo_dir/apps/flutter/app/android/app/src/main/assets/android-runtime"
jni_libs_dir="$repo_dir/apps/flutter/app/android/app/src/main/jniLibs"

ndk_dir="${ANDROID_NDK_HOME:?ANDROID_NDK_HOME must point to android-ndk-r29-beta4 in Fedora WSL}"
toolchain_dir="$ndk_dir/toolchains/llvm/prebuilt/linux-x86_64/bin"
api_level=23
bash_api_level=24

busybox_src="$source_dir/busybox-1.38.0"
proot_src="$source_dir/proot-5.4.0"
proot_patch="$runtime_dir/patches/proot-5.4.0-operit-android.patch"
talloc_src="$source_dir/talloc-2.4.3"
bash_src="$source_dir/bash-5.2.37"

require_path() {
    local path="$1"
    test -e "$path" || {
        echo "Required path does not exist: $path" >&2
        exit 1
    }
}

set_config_value() {
    local file="$1"
    local key="$2"
    local value="$3"
    local tmp
    tmp="$(mktemp)"
    awk -v key="$key" -v value="$value" '
        $0 == "# " key " is not set" || index($0, key "=") == 1 {
            print key "=" value
            seen = 1
            next
        }
        { print }
        END {
            if (seen != 1) {
                print key "=" value
            }
        }
    ' "$file" > "$tmp"
    mv "$tmp" "$file"
}

set_config_disabled() {
    local file="$1"
    local key="$2"
    local tmp
    tmp="$(mktemp)"
    awk -v key="$key" '
        $0 == "# " key " is not set" || index($0, key "=") == 1 {
            print "# " key " is not set"
            seen = 1
            next
        }
        { print }
        END {
            if (seen != 1) {
                print "# " key " is not set"
            }
        }
    ' "$file" > "$tmp"
    mv "$tmp" "$file"
}

write_talloc_config_h() {
    local output_dir="$1"
    cat > "$output_dir/config.h" <<'EOF'
#define HAVE_STDLIB_H 1
#define HAVE_STDARG_H 1
#define HAVE_INTTYPES_H 1
#define HAVE_STDINT_H 1
#define HAVE_UNISTD_H 1
#define HAVE_STRING_H 1
#define HAVE_STRINGS_H 1
#define HAVE_SYS_TYPES_H 1
#define HAVE_SYS_STAT_H 1
#define HAVE_SYS_PARAM_H 1
#define HAVE_LIMITS_H 1
#define HAVE_STDBOOL_H 1
#define HAVE_MALLOC_H 1
#define HAVE_ERRNO_DECL 1
#define HAVE_VA_COPY 1
#define HAVE_C99_VSNPRINTF 1
#define HAVE_SNPRINTF 1
#define HAVE_VSNPRINTF 1
#define HAVE_ASPRINTF 1
#define HAVE_VASPRINTF 1
#define HAVE_DPRINTF 1
#define HAVE_VDPRINTF 1
#define HAVE_DLFCN_H 1
#define HAVE_DLERROR 1
#define HAVE_DLOPEN 1
#define HAVE_DLSYM 1
#define HAVE_DLCLOSE 1
#define HAVE_STRDUP 1
#define HAVE_STRNDUP 1
#define HAVE_STRNLEN 1
#define HAVE_MEMMOVE 1
#define HAVE_MKTIME 1
#define HAVE_TIMEGM 1
#define HAVE_UTIME 1
#define HAVE_UTIMES 1
#define HAVE_SETENV 1
#define HAVE_UNSETENV 1
#define HAVE_SETEUID 1
#define HAVE_SETEGID 1
#define HAVE_CHOWN 1
#define HAVE_CHROOT 1
#define HAVE_LINK 1
#define HAVE_READLINK 1
#define HAVE_SYMLINK 1
#define HAVE_REALPATH 1
#define HAVE_LCHOWN 1
#define HAVE_FTRUNCATE 1
#define HAVE_SETLINEBUF 1
#define HAVE_STRCASESTR 1
#define HAVE_STRSEP 1
#define HAVE_STRTOK_R 1
#define HAVE_STRTOLL 1
#define HAVE_STRTOULL 1
#define HAVE_INITGROUPS 1
#define HAVE_SOCKETPAIR 1
#define HAVE_POLL 1
#define HAVE_USLEEP 1
#define HAVE_PREAD 1
#define HAVE_PWRITE 1
#define HAVE_CONNECT 1
#define HAVE_GETHOSTBYNAME 1
#define HAVE_GETIFADDRS 1
#define HAVE_FREEIFADDRS 1
#define HAVE_CLOCK_GETTIME 1
#define HAVE_INET_NTOA 1
#define HAVE_INET_PTON 1
#define HAVE_INET_NTOP 1
#define HAVE_INET_ATON 1
#define HAVE_SECURE_MKSTEMP 1
#define HAVE_MKDTEMP 1
#define HAVE_INTPTR_T 1
#define HAVE_UINTPTR_T 1
#define HAVE_PTRDIFF_T 1
#define HAVE_FUNCTION_MACRO 1
#define HAVE_VOLATILE 1
#define HAVE_BOOL 1
#define HAVE_CONSTRUCTOR_ATTRIBUTE 1
#define HAVE_VISIBILITY_ATTR 1
#define HAVE_FALLTHROUGH_ATTRIBUTE 1
#define HAVE___THREAD 1
#define HAVE_DECL_EWOULDBLOCK 1
#define HAVE_DECL_ENVIRON 1
#define HAVE_STRERROR 1
#define STRERROR_R_XSI_NOT_GNU 1
#define HAVE_STRERROR_R 1
#define HAVE_FDATASYNC 1
#define HAVE_DECL_FDATASYNC 1
EOF
}

write_talloc_pc() {
    local output_dir="$1"
    local install_dir="$2"
    mkdir -p "$install_dir/lib/pkgconfig"
    cat > "$install_dir/lib/pkgconfig/talloc.pc" <<EOF
prefix=$install_dir
exec_prefix=\${prefix}
libdir=\${prefix}/lib
includedir=\${prefix}/include

Name: talloc
Description: hierarchical pool based memory allocator
Version: 2.4.3
Libs: -L\${libdir} -ltalloc -ldl
Cflags: -I\${includedir}
EOF
}

if [ -n "${OPERIT_ANDROID_RUNTIME_ABIS:-}" ]; then
    read -r -a abis <<< "$OPERIT_ANDROID_RUNTIME_ABIS"
else
    abis=(arm64-v8a armeabi-v7a x86_64)
fi

if [ -n "${OPERIT_ANDROID_RUNTIME_COMPONENTS:-}" ]; then
    read -r -a components <<< "$OPERIT_ANDROID_RUNTIME_COMPONENTS"
else
    components=(busybox talloc proot bash)
fi

has_component() {
    local requested="$1"
    local component
    for component in "${components[@]}"; do
        if [ "$component" = "$requested" ]; then
            return 0
        fi
    done
    return 1
}

compiler_for_abi() {
    case "$1" in
        arm64-v8a) echo "$toolchain_dir/aarch64-linux-android${api_level}-clang" ;;
        armeabi-v7a) echo "$toolchain_dir/armv7a-linux-androideabi${api_level}-clang" ;;
        x86_64) echo "$toolchain_dir/x86_64-linux-android${api_level}-clang" ;;
        *) echo "Unsupported ABI: $1" >&2; exit 1 ;;
    esac
}

bash_compiler_for_abi() {
    case "$1" in
        arm64-v8a) echo "$toolchain_dir/aarch64-linux-android${bash_api_level}-clang" ;;
        armeabi-v7a) echo "$toolchain_dir/armv7a-linux-androideabi${bash_api_level}-clang" ;;
        x86_64) echo "$toolchain_dir/x86_64-linux-android${bash_api_level}-clang" ;;
        *) echo "Unsupported ABI: $1" >&2; exit 1 ;;
    esac
}

bash_cxx_for_abi() {
    case "$1" in
        arm64-v8a) echo "$toolchain_dir/aarch64-linux-android${bash_api_level}-clang++" ;;
        armeabi-v7a) echo "$toolchain_dir/armv7a-linux-androideabi${bash_api_level}-clang++" ;;
        x86_64) echo "$toolchain_dir/x86_64-linux-android${bash_api_level}-clang++" ;;
        *) echo "Unsupported ABI: $1" >&2; exit 1 ;;
    esac
}

bash_host_for_abi() {
    case "$1" in
        arm64-v8a) echo aarch64-linux-android ;;
        armeabi-v7a) echo arm-linux-androideabi ;;
        x86_64) echo x86_64-linux-android ;;
        *) echo "Unsupported ABI: $1" >&2; exit 1 ;;
    esac
}

busybox_arch_for_abi() {
    case "$1" in
        arm64-v8a) echo arm64 ;;
        armeabi-v7a) echo arm ;;
        x86_64) echo x86_64 ;;
        *) echo "Unsupported ABI: $1" >&2; exit 1 ;;
    esac
}

patch_bash_source() {
    local work_src="$1"
    python3 - "$work_src" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])

tparam = root / "lib/termcap/tparam.c"
text = tparam.read_text()
old = '#include "ltcap.h"'
new = '#if defined (HAVE_UNISTD_H)\n#include <unistd.h>\n#endif\n\n#include "ltcap.h"'
if new not in text:
    if old not in text:
        raise SystemExit(f"patch target not found: {tparam}")
    tparam.write_text(text.replace(old, new, 1))

bashline = root / "bashline.c"
text = bashline.read_text()
old = "#if defined (__WIN32__) || defined (__OPENNT) || !defined (HAVE_GRP_H)"
new = "#if defined (__WIN32__) || defined (__OPENNT) || !defined (HAVE_GRP_H) || (defined (__ANDROID_API__) && __ANDROID_API__ < 26)"
if new not in text:
    if old not in text:
        raise SystemExit(f"patch target not found: {bashline}")
    bashline.write_text(text.replace(old, new, 1))

shmbutil = root / "include/shmbutil.h"
text = shmbutil.read_text()
old = "#define MBLEN(s, n)\t((MB_CUR_MAX > 1) ? mblen ((s), (n)) : 1)"
new = "#if defined (__ANDROID_API__) && __ANDROID_API__ < 26\n#define MBLEN(s, n)\t((MB_CUR_MAX > 1) ? mbrlen ((s), (n), 0) : 1)\n#else\n#define MBLEN(s, n)\t((MB_CUR_MAX > 1) ? mblen ((s), (n)) : 1)\n#endif"
if new not in text:
    if old not in text:
        raise SystemExit(f"patch target not found: {shmbutil}")
    shmbutil.write_text(text.replace(old, new, 1))

for relative in ("lib/sh/mbscasecmp.c", "lib/sh/mbscmp.c", "locale.c"):
    source = root / relative
    text = source.read_text()
    changed = text.replace("mblen ((char *)NULL, 0)", "mbrlen ((char *)NULL, 0, 0)")
    changed = changed.replace("mblen ((char *) NULL, 0)", "mbrlen ((char *) NULL, 0, 0)")
    if changed != text:
        source.write_text(changed)
    elif "mbrlen ((char *)NULL, 0, 0)" not in text and "mbrlen ((char *) NULL, 0, 0)" not in text:
        raise SystemExit(f"patch target not found: {source}")
PY
}

build_busybox() {
    local abi="$1"
    local cc="$2"
    local out_dir="$build_dir/busybox/$abi"
    local arch
    arch="$(busybox_arch_for_abi "$abi")"

    rm -rf "$out_dir"
    mkdir -p "$out_dir"

    make -C "$busybox_src" O="$out_dir" android2_defconfig \
        ARCH="$arch" \
        CROSS_COMPILE= \
        CC="$cc" \
        AR="$toolchain_dir/llvm-ar" \
        NM="$toolchain_dir/llvm-nm" \
        STRIP="$toolchain_dir/llvm-strip" \
        OBJCOPY="$toolchain_dir/llvm-objcopy" \
        OBJDUMP="$toolchain_dir/llvm-objdump" \
        HOSTCC=gcc
    set_config_value "$out_dir/.config" "CONFIG_CROSS_COMPILER_PREFIX" '""'
    set_config_value "$out_dir/.config" "CONFIG_EXTRA_CFLAGS" '"-D__ANDROID__ -DANDROID"'
    set_config_value "$out_dir/.config" "CONFIG_SYSROOT" '""'
    set_config_value "$out_dir/.config" "CONFIG_PREFIX" "\"$out_dir/install\""
    set_config_value "$out_dir/.config" "CONFIG_PIE" "y"
    set_config_value "$out_dir/.config" "CONFIG_USE_BB_CRYPT" "y"
    set_config_value "$out_dir/.config" "CONFIG_USE_BB_CRYPT_SHA" "y"
    set_config_disabled "$out_dir/.config" "CONFIG_STATIC"
    set_config_disabled "$out_dir/.config" "CONFIG_STATIC_LIBGCC"
    set_config_disabled "$out_dir/.config" "CONFIG_FEATURE_SYNC_FANCY"
    set_config_disabled "$out_dir/.config" "CONFIG_SWAPON"
    set_config_disabled "$out_dir/.config" "CONFIG_SWAPOFF"
    set_config_disabled "$out_dir/.config" "CONFIG_SEEDRNG"
    set_config_disabled "$out_dir/.config" "CONFIG_TC"

    set +o pipefail
    yes "" | make -C "$busybox_src" O="$out_dir" oldconfig \
        ARCH="$arch" \
        CROSS_COMPILE= \
        CC="$cc" \
        AR="$toolchain_dir/llvm-ar" \
        NM="$toolchain_dir/llvm-nm" \
        STRIP="$toolchain_dir/llvm-strip" \
        OBJCOPY="$toolchain_dir/llvm-objcopy" \
        OBJDUMP="$toolchain_dir/llvm-objdump" \
        HOSTCC=gcc
    oldconfig_status="${PIPESTATUS[1]}"
    set -o pipefail
    test "$oldconfig_status" -eq 0
    make -C "$busybox_src" O="$out_dir" -j"$(nproc)" \
        ARCH="$arch" \
        CROSS_COMPILE= \
        CC="$cc" \
        AR="$toolchain_dir/llvm-ar" \
        NM="$toolchain_dir/llvm-nm" \
        STRIP="$toolchain_dir/llvm-strip" \
        OBJCOPY="$toolchain_dir/llvm-objcopy" \
        OBJDUMP="$toolchain_dir/llvm-objdump" \
        HOSTCC=gcc

    mkdir -p "$jni_libs_dir/$abi"
    cp "$out_dir/busybox" "$jni_libs_dir/$abi/liboperit_busybox.so"
    "$toolchain_dir/llvm-strip" "$jni_libs_dir/$abi/liboperit_busybox.so"
}

build_talloc() {
    local abi="$1"
    local cc="$2"
    local out_dir="$build_dir/talloc/$abi"
    local install_dir="$out_dir/install"

    rm -rf "$out_dir"
    mkdir -p "$out_dir" "$install_dir/lib" "$install_dir/include"
    write_talloc_config_h "$out_dir"

    "$cc" -O2 -fPIC -fvisibility=hidden \
        -D__STDC_WANT_LIB_EXT1__=1 \
        -DTALLOC_BUILD_VERSION_MAJOR=2 \
        -DTALLOC_BUILD_VERSION_MINOR=4 \
        -DTALLOC_BUILD_VERSION_RELEASE=3 \
        -I"$out_dir" \
        -I"$talloc_src" \
        -I"$talloc_src/lib/replace" \
        -c "$talloc_src/talloc.c" \
        -o "$out_dir/talloc.o"

    "$toolchain_dir/llvm-ar" rcs "$install_dir/lib/libtalloc.a" "$out_dir/talloc.o"
    cp "$talloc_src/talloc.h" "$install_dir/include/talloc.h"
    write_talloc_pc "$out_dir" "$install_dir"
}

build_proot() {
    local abi="$1"
    local cc="$2"
    local out_dir="$build_dir/proot/$abi"
    local work_src="$build_dir/proot-source/$abi"
    local talloc_install="$build_dir/talloc/$abi/install"

    rm -rf "$out_dir" "$work_src"
    mkdir -p "$out_dir" "$work_src"
    cp -a "$proot_src/." "$work_src"
    require_path "$proot_patch"
    patch -d "$work_src" -p1 --batch < "$proot_patch"
    awk '
        /^LOADER_LDFLAGS\$1 \+= -static -nostdlib / {
            sub(/-Ttext=\$\(LOADER_ADDRESS\$1\)/, "--image-base=$(LOADER_ADDRESS$1)")
            sub(/-z,noexecstack$/, "-z,noexecstack,-z,max-page-size=16384,-z,common-page-size=16384")
        }
        { print }
    ' "$work_src/src/GNUmakefile" > "$out_dir/GNUmakefile"
    mv "$out_dir/GNUmakefile" "$work_src/src/GNUmakefile"
    awk '
        { print }
        /^#include "cli\/note.h"/ {
            print ""
            print "#if defined(__ANDROID__) && !defined(__LP64__)"
            print "#define prlimit prlimit64"
            print "#define rlimit rlimit64"
            print "#endif"
        }
    ' "$work_src/src/syscall/rlimit.c" > "$out_dir/rlimit.c"
    mv "$out_dir/rlimit.c" "$work_src/src/syscall/rlimit.c"

    (
        PKG_CONFIG_PATH="$talloc_install/lib/pkgconfig" \
        make -C "$work_src/src" \
            CC="$cc" \
            LD="$cc" \
            STRIP="$toolchain_dir/llvm-strip" \
            OBJCOPY="$toolchain_dir/llvm-objcopy" \
            OBJDUMP="$toolchain_dir/llvm-objdump" \
            V=1 \
            build.h loader.elf

        PKG_CONFIG_PATH="$talloc_install/lib/pkgconfig" \
        make -C "$work_src/src" \
            CC="$cc" \
            LD="$cc" \
            STRIP="$toolchain_dir/llvm-strip" \
            OBJCOPY="$toolchain_dir/llvm-objcopy" \
            OBJDUMP="$toolchain_dir/llvm-objdump" \
            V=1 \
            proot
    )

    mkdir -p "$jni_libs_dir/$abi"
    cp "$work_src/src/proot" "$out_dir/proot"
    cp "$work_src/src/loader.elf" "$out_dir/loader"
    cp "$out_dir/proot" "$jni_libs_dir/$abi/liboperit_proot.so"
    cp "$out_dir/loader" "$jni_libs_dir/$abi/liboperit_loader.so"
    "$toolchain_dir/llvm-strip" "$jni_libs_dir/$abi/liboperit_proot.so"
    "$toolchain_dir/llvm-strip" "$jni_libs_dir/$abi/liboperit_loader.so"
}

build_bash() {
    local abi="$1"
    local cc="$2"
    local cxx
    local host
    local out_dir="$build_dir/bash/$abi"
    local work_src="$build_dir/bash-source/$abi"
    local tls_src="$out_dir/operit_tls_align_dummy.c"
    local tls_obj="$out_dir/operit_tls_align_dummy.o"

    cxx="$(bash_cxx_for_abi "$abi")"
    host="$(bash_host_for_abi "$abi")"
    require_path "$cxx"

    rm -rf "$out_dir" "$work_src"
    mkdir -p "$out_dir" "$work_src"
    cp -a "$bash_src/." "$work_src"
    patch_bash_source "$work_src"

    cat > "$tls_src" <<'EOF'
__thread char operit_tls_alignment_dummy __attribute__((aligned(64), used));
EOF
    "$cc" -O2 -fno-emulated-tls -c "$tls_src" -o "$tls_obj"

    (
        cd "$work_src"
        PATH="$toolchain_dir:$PATH" \
        CC="$cc" \
        CXX="$cxx" \
        CPP="$cc -E" \
        AR="$toolchain_dir/llvm-ar" \
        RANLIB="$toolchain_dir/llvm-ranlib" \
        STRIP="$toolchain_dir/llvm-strip" \
        CC_FOR_BUILD=gcc \
        CFLAGS="-O2" \
        CPPFLAGS="" \
        LDFLAGS="-static -Wl,-z,max-page-size=16384 -Wl,-z,common-page-size=16384" \
        CFLAGS_FOR_BUILD="-g -DCROSS_COMPILING -std=gnu89" \
        ./configure \
            --host="$host" \
            --build="$(uname -m)-pc-linux-gnu" \
            --prefix=/usr \
            --without-bash-malloc \
            --enable-multibyte \
            --enable-progcomp \
            bash_cv_job_control_missing=present \
            bash_cv_sys_siglist=yes \
            bash_cv_func_sigsetjmp=present \
            bash_cv_unusable_rtsigs=no \
            ac_cv_func_getentropy=no \
            ac_cv_func_mbsnrtowcs=no \
            ac_cv_func_getrandom=no \
            ac_cv_func_getpwent=no \
            bash_cv_dev_fd=whacky \
            bash_cv_getcwd_malloc=yes

        PATH="$toolchain_dir:$PATH" \
        make -j"$(nproc)" \
            CC_FOR_BUILD=gcc \
            CFLAGS_FOR_BUILD="-g -DCROSS_COMPILING -std=gnu89" \
            LDFLAGS="-static -Wl,-z,max-page-size=16384 -Wl,-z,common-page-size=16384 $tls_obj"
    )

    mkdir -p "$jni_libs_dir/$abi"
    cp "$work_src/bash" "$out_dir/libbash.so.unstripped"
    cp "$work_src/bash" "$jni_libs_dir/$abi/libbash.so"
    "$toolchain_dir/llvm-strip" "$jni_libs_dir/$abi/libbash.so"
}

require_path "$toolchain_dir"
require_path "$busybox_src"
require_path "$proot_src"
require_path "$talloc_src"
require_path "$bash_src"

mkdir -p "$asset_dir" "$jni_libs_dir"

for abi in "${abis[@]}"; do
    cc="$(compiler_for_abi "$abi")"
    require_path "$cc"
    if has_component busybox; then
        build_busybox "$abi" "$cc"
    fi
    if has_component talloc || has_component proot; then
        build_talloc "$abi" "$cc"
    fi
    if has_component proot; then
        build_proot "$abi" "$cc"
    fi
    if has_component bash; then
        bash_cc="$(bash_compiler_for_abi "$abi")"
        require_path "$bash_cc"
        build_bash "$abi" "$bash_cc"
    fi
done
