# iOS Runtime Tooling

The active iOS terminal runs an x86 Alpine Linux environment through
[iSH](ish/), including Node.js 24, Python, and the shared app-storage mounts.
Source preparation, patches, rootfs generation, and static-library builds live
in `ish/`.

The unused Toybox, Node Mobile, and BeeWare CPython build implementations have
been removed. iOS builds use only the iSH runtime pipeline.
