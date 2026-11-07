#!/system/bin/sh
# Opens only the Flutter Activity registered from this module's system mount.
MODDIR=${0%/*}
exec "$MODDIR/bin/operit-host-ctl" open
