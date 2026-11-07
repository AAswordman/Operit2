#!/system/bin/sh
# Starts the module supervisor without blocking Android boot.
MODDIR=${0%/*}
umask 077
mkdir -p /data/adb/operit2/users/0/logs
"$MODDIR/bin/operit-host-ctl" start >>/data/adb/operit2/users/0/logs/supervisor.log 2>&1
