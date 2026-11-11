# operit-vfs

Shared filesystem mapping and runtime-bound file references, independent of tool
execution and application services.

- Internal `PathMapper`: private virtual namespace and backend resolution implementation.
- `MountRegistry`: mount catalog persisted through its explicitly supplied storage host.
- `VisualFileSystem`: file operations that translate locators only at backend dispatch.
- `VfsPath`: validated, serializable absolute virtual addresses.
- `VfsFile`: a file reference retaining the owning filesystem. It implements the
  `FileSource` and `FileTarget` contracts without publishing its resolved locator.

Consumers obtain the filesystem from runtime host assembly and pass file references
to capabilities such as installation, OCR and HTTP download. Hosts may read/write
through the reference. A platform implementation requiring native interoperation
can request `nativePath()`; document mounts explicitly reject that operation.
External crates cannot construct or retrieve a mapper. `VfsPath` exposes virtual
address arithmetic; filesystem operations own conversion to backend locators.

Node selection and relative tool invocation context belong to the tools layer.
The VFS has no dependency on tool names, remote routing, chat state or HostManager.

See [VFS and host boundaries](../../../../docs/vfs-host-boundaries.md).
