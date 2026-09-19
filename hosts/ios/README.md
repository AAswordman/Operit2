# iOS host

## Terminal storage and paths

iSH imports the bundled Alpine filesystem once into
`Library/Application Support/operit-ish/rootfs` in the application data container.
Subsequent launches reuse it. Files in `/root`, shell configuration, and software
installed into `/usr` survive process restarts and app updates that preserve app
data. Removing app data also removes this filesystem.

Host directories are shared through hostfs **at their original absolute paths**,
matching Android's same-path bind mounts. The Documents directory and Operit2
application-support directory are available to every shell; a host working
directory can also be mounted on session creation. The shell and native file
tools read and write the same files immediately. Mounts are reestablished on
launch; no workspace copy or hash-based MCP path is involved. Linux-only paths
such as `/root` belong to the persistent Alpine filesystem.

On a device, host paths start with the app's `/var/mobile/...` container path;
simulators use `/Users/...`. iOS can change the container UUID during an update.
The native bootstrap channel restores persisted runtime/workspace roots to the
current container. Scripts should use `$HOME` for terminal-owned files and the
current absolute workspace path supplied by the host for shared files.

## Serial transport

Core opens serial endpoints through `SerialPortHost`. The iOS Host currently
reports that no native serial accessory provider is installed. Implementing an
accessory provider and registering it in the host manager enables the existing
Link framing and pairing protocol without platform branches in Core. This does
not advertise unrestricted USB/UART access on iOS.
