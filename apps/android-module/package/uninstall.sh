#!/system/bin/sh
# Stops the Core and preserves user data; removing the module removes its mount on reboot.
MODDIR=${0%/*}
"$MODDIR/bin/operit-host-ctl" stop
