#!/system/bin/sh
# Deploys one shared Android artifact for the privileged Host and native Flutter windows.
[ "$ARCH" = arm64 ] || abort "This Operit trial requires ARM64 Android."
MOUNT_PROVIDER=
for candidate in /data/adb/modules/*; do
    [ -f "$candidate/module.prop" ] || continue
    [ ! -f "$candidate/disable" ] && [ ! -f "$candidate/remove" ] || continue
    [ -f "$candidate/metamount.sh" ] || continue
    if grep -Eq '^metamodule=(1|true)$' "$candidate/module.prop"; then
        MOUNT_PROVIDER=$candidate
        break
    fi
done
[ -n "$MOUNT_PROVIDER" ] || abort "Install and enable a KernelSU mounting metamodule (for example meta-overlayfs), reboot, then install Operit."
APK=$MODPATH/payload/operit-android.apk
[ -f "$APK" ] || abort "Missing shared Android Host APK."
ui_print "Operit2 Android privileged Host deployment"
ui_print "Mount provider: ${MOUNT_PROVIDER##*/}"
ui_print "Reboot required. No APK installation command will be executed."
# Verify compressed inputs before expanding only the files named by the build manifest.
(cd "$MODPATH" && sha256sum -c payload/archive.sha256) || abort "Module payload checksum mismatch."
HOSTDIR=$MODPATH/system/app/Operit
mkdir -p "$HOSTDIR/lib/arm64" "$MODPATH/runtime/assets" || abort "Cannot prepare Host deployment directories."
while IFS= read -r library; do
    case "$library" in lib*.so) ;; *) abort "Invalid Host library entry: $library" ;; esac
    case "$library" in *[!a-zA-Z0-9_.+-]*) abort "Invalid Host library filename: $library" ;; esac
    unzip -p "$APK" "lib/arm64-v8a/$library" > "$HOSTDIR/lib/arm64/$library" || abort "Cannot extract Host library: $library"
done < "$MODPATH/payload/native-libraries.list"
unzip -p "$APK" assets/android-runtime/arm64-v8a/rootfs.tar.gz.bin > "$MODPATH/runtime/assets/rootfs.tar.gz" || abort "Cannot extract Host terminal environment."
unzip -p "$APK" assets/android-runtime/arm64-v8a/rootfs.tar.gz.bin.sha256 > "$MODPATH/runtime/assets/rootfs.tar.gz.sha256" || abort "Cannot extract Host environment checksum."
mv "$APK" "$HOSTDIR/Operit.apk" || abort "Cannot publish Android Host APK."
(cd "$MODPATH" && sha256sum -c payload/installed.sha256) || abort "Expanded Host checksum mismatch."
set_perm_recursive "$MODPATH" 0 0 0755 0644
set_perm_recursive "$MODPATH/bin" 0 0 0755 0755
set_perm_recursive "$MODPATH/system" 0 0 0755 0644 u:object_r:system_file:s0
set_perm_recursive "$MODPATH/system/app/Operit/lib" 0 0 0755 0755 u:object_r:system_file:s0
set_perm "$MODPATH/service.sh" 0 0 0755
set_perm "$MODPATH/action.sh" 0 0 0755
set_perm "$MODPATH/uninstall.sh" 0 0 0755
