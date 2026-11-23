# Super Admin

`super_admin` owns terminal commands, profile discovery, screen reads, input and
explicit close. Its tool names and parameters are unchanged. The built-in source
is now a ToolPkg with a main runtime, allowing cleanup to continue without another
AI invocation. Package synchronization replaces the old `super_admin.js` asset
with `super_admin.toolpkg`.

## Idle sessions

The plugin registers a platform-neutral Host interval of one minute. A terminal
acquired by this plugin becomes eligible for automatic close after **10 minutes**
without plugin activity. Actual cleanup occurs on the next delivered clock event;
hosts that suspend event delivery perform cleanup after delivery resumes.

- Command execution, screen reads and input reset the inactivity deadline.
- Acquisition holds a family lease; command streaming then holds a lease for
  the actual session. Other idle generations can still be reclaimed. Queued and running
  plugin commands remain protected, including silent commands and package calls
  that have returned an `execution_id` while execution continues.
- Before cleanup, the plugin reads the Host's running state and calls
  `Tools.System.terminal.closeIfIdle` with the exact profile-scoped session family.
  The Host atomically checks ownership and queued/running work. A rejection is
  logged; automatic cleanup never invokes unconditional close.
- Explicit close removes the plugin's record. Automatic close releases shell
  context; the next command creates a fresh session.
- Workspace terminals created directly by users and other plugins are excluded.
  Direct workspace input into an AI-created terminal does not update this
  plugin's activity record; only managed commands reported as running by the Host
  are protected through that route. Shell background subprocesses are not
  represented by the managed-command running flag.

The main runtime owns the in-memory records and serializes cleanup with tool
activity through IPC. Records are local to the current runtime and are discarded
when the plugin is unloaded or replaced. Terminals created before that runtime
are not discovered or retrospectively reclaimed.

## Checks

Install TypeScript 5.9.2 under `.ci-tools/typescript`, then run from the repository
root:

```sh
node --test tools/tests/super_admin_terminal_routing.test.mjs tools/tests/clean_on_exit_output.test.mjs tools/tests/terminal_streaming.test.mjs
node .ci-tools/typescript/node_modules/typescript/bin/tsc --noEmit -p plugins/packages/buildin/super_admin/tsconfig.json
```

The tests evaluate production sources in memory and advance a simulated clock;
they do not launch shells or build the application.
