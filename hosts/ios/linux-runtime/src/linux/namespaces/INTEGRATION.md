# Namespace integration contract

This module is not wired into syscall dispatch, procfs, or process lifecycle by
itself. Add `namespaces.cpp` to the runtime and dispatch AArch64 unshare (97)
and setns (268) with the contexts in `namespaces.h`.

## Ownership and transactions

- Create one `UserNamespaceIdentity` for the guest's initial user namespace,
  and initialize its first mount identity with `acquire_mount_namespace`.
  Keep both identities on the calling task for as long as it belongs to them,
  even if no namespace FD is open. Ordinary fork/clone and exec retain them.
- Call `check_clone(request.flags, guest_cap_sys_admin)` before the clone flag
  whitelist and before publishing a child. Reject unsupported namespace flags;
  `CLONE_NEWNS | CLONE_FS` is invalid. For a permitted `CLONE_NEWNS`, initialize
  the child's inherited filesystem and private fs context, then call
  `unshare(clone_newns, child_context)` before publishing it.
- For unshare, `prepare_fs` must stage a private fs context; when passed new
  mounts it must also rebase pinned cwd/root mount references without resolving
  their old path strings. Return null if renamed or detached paths cannot be
  preserved. `CLONE_FILES` is staged by the module. Publish the filesystem, FD
  table, fs context and namespace identity as a serialized transaction.
- `Syscalls` is currently shared by the thread group. Until these resources are
  task-scoped, set `task_scoped` only when that group has one live task; reject
  unsafe changes for larger groups. `fs_private` for setns must count all
  fs-context users, including other processes, not just tasks in the group.
  Serialize the whole operation against clone, exec, and resource mutations.

## Namespace FDs and setns

- Only a procfs namespace-open implementation that has verified target access
  and its actual task/user/mount identities may call `open_mount_namespace`.
  Pass the target's existing mount identity together with its filesystem; the
  module rejects null or mismatched identities and never creates one on open.
  Dup and fork retain the open description and its namespace identity. Existing
  SCM_RIGHTS rejects virtual FDs; keep rejecting namespace FDs unless that path
  is extended to preserve the same description. `/proc/self/fd` reopening must
  likewise retain a genuine namespace attachment or reject the open.
- A setns mount-install hook must install only the target mount namespace,
  preserve the caller's own virtual contexts, and set cwd/root to the target
  namespace root. Replacing the entire target `Filesystem` is unsafe because
  it also imports that process's virtual callbacks. The module rejects setns
  when the real FD, caller identity, capability, exclusive fs context, or safe
  install hook is missing.
- Capability booleans are guest capabilities in the relevant guest user
  namespace, never the host UID. The supported setns subset requires the exact
  owning user-namespace identity and both CAP_SYS_ADMIN and CAP_SYS_CHROOT;
  ancestry and pidfd-based setns are not implemented. The current guest
  capability set is empty, so privileged operations must fail with EPERM.

## Tests

`tests/namespaces.cpp` exercises staged isolation and identity directly.
`tests/namespaces_integration.py --runtime <wired-runtime>` builds and runs the
guest fixture for one and two CPUs. The positive setns unit fixture replaces a
whole `Filesystem` only because its roots and virtual contexts are identical;
it is not a production implementation of `prepare_mount_install`.
