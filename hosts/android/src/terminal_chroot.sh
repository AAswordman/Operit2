set -eu
rootfs=$1
workdir=$2
prompt=$3
mode=$4
command=$5
shift 5

# Requires Root and private propagation before mounting anything.
[ "$(/system/bin/id -u)" = 0 ] || exit 126
/system/bin/mount -o rprivate none /
[ "$rootfs" != / ] && [ "$(/system/bin/realpath "$rootfs")" = "$rootfs" ] || exit 125

# Reject guest symlinks component by component, before creating a mount point.
guest_directory() {
    destination=$rootfs
    remaining=${1#/}
    while [ -n "$remaining" ]; do
        component=${remaining%%/*}
        case "$component" in
            ''|.|..) return 125 ;;
        esac
        destination=$destination/$component
        if [ -L "$destination" ]; then
            printf 'chroot mount target is a symlink: %s\n' "$1" >&2
            return 125
        fi
        if [ ! -d "$destination" ]; then
            /system/bin/mkdir -- "$destination" || return
        fi
        case "$remaining" in
            */*) remaining=${remaining#*/} ;;
            *) remaining= ;;
        esac
    done
}

while [ "$#" -gt 0 ]; do
    [ -d "$1" ] || { printf 'chroot mount source is not a directory: %s\n' "$1" >&2; exit 125; }
    guest_directory "$2"
    /system/bin/mount -o "$3" "$1" "$destination"
    shift 3
done

if [ "$mode" = command ]; then
    set -- -c "$command"
else
    set -- -i
fi

exec /system/bin/chroot "$rootfs" /usr/bin/env -i \
    HOME=/root TERM=xterm-256color LANG=C.UTF-8 'PS1=$PWD $ ' \
    PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    "PROMPT_COMMAND=$prompt" /bin/bash --noprofile --norc -c \
    'cd -- "$1" || exit; shift; exec /bin/bash --noprofile --norc "$@"' operit-chroot "$workdir" "$@"
