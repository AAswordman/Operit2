# CLI workspaces

A workspace is a persisted record with an ID, a display name, and an ordered list
of named VFS folder mounts. Chats bind to the workspace ID. A directory path is
an input to mounting, never a workspace ID. Several chats can share a workspace;
folder edits apply to all of them. Workspaces exist independently of chats and
appear in `workspace list` before any chat binds to them.

## Create, mount, then bind

These shell examples use `jq` to read the structured CLI result:

```sh
WORKSPACE_ID=$(operit2 cli workspace create "Task workspace" --json | jq -r .workspaceId)
operit2 cli workspace mount "$WORKSPACE_ID" project --native "$PWD" --primary
CHAT_ID=$(operit2 cli chat new --workspace "$WORKSPACE_ID" --json | jq -r .chatId)
```

Creating without a directory initializes a blank app-owned `default` folder at
`/operitapp/workspaces/<generated-directory-id>`. The returned folder metadata
contains its address; the workspace ID identifies the record. Mounting `project`
adds the existing directory to that record. `--primary` makes it the default working directory
for the chat and workspace commands. The app-owned folder remains available.

For an existing chat, binding is a separate operation:

```sh
operit2 cli workspace bind "$CHAT_ID" "$WORKSPACE_ID"
operit2 cli workspace unbind "$CHAT_ID"
```

The common current-folder flow also has a direct creation form. It creates the
workspace with that directory as its only mount, named `default`, and leaves
the existing files intact:

```sh
WORKSPACE_ID=$(operit2 cli workspace create "Task workspace" --native "$PWD" --json | jq -r .workspaceId)
CHAT_ID=$(operit2 cli chat new --workspace "$WORKSPACE_ID" --json | jq -r .chatId)
```

The TUI's current-folder prompt uses the shared chat folder-binding operation,
which creates a workspace record when the chat has none and stores its ID on the chat.

## Native and virtual directory inputs

`--native` takes an absolute path on the CLI host, including native directories
whose names overlap VFS roots. In a Linux container, `--native /app` mounts
`/virtual/linux/app`; native `/operitapp`, `/mnt`, `/virtual`, `/workspace`, and
`/` are also interpreted as host directories. On Windows,
`--native 'D:\Projects\demo'` maps through `/virtual/windows/d/Projects/demo`.
The conversion uses the host filesystem declaration shared with VFS resolution.

`--vfs` takes an explicit workspace VFS address. It supports mounted directories
and node-qualified addresses such as `node-id://virtual/linux/projects/demo`.
It never guesses a native interpretation for an unknown virtual root.

```sh
operit2 cli workspace mount "$WORKSPACE_ID" assets --vfs /virtual/linux/shared/assets
operit2 cli workspace primary "$WORKSPACE_ID" assets
operit2 cli workspace unmount "$WORKSPACE_ID" assets
```

Mounts are stored with their owning node. The same name and directory can be
mounted repeatedly. As in the graphical folder picker, mounting an existing
path preserves its current name, and mounting a different path under an
existing name replaces that folder's path. `--primary` selects the mounted path,
including when it already has a different name.
Unmounting preserves directory contents; a workspace must retain at least one
folder. Removing the primary folder makes the next folder primary.

CLI startup carries the native caller directory as a VFS execution context for
relative file arguments. It does not create or bind a workspace. Commands such
as `chat new`, `model list`, and `workspace list` can run from native `/app`
without setting up a workspace first. Local dispatch and daemon dispatch carry
the same caller directory.

## Inspect, execute, and delete

```sh
operit2 cli workspace list --json
operit2 cli workspace show "$WORKSPACE_ID" --json
operit2 cli workspace chats "$WORKSPACE_ID" --json
operit2 cli workspace commands "$WORKSPACE_ID"
operit2 cli workspace run "$WORKSPACE_ID" build
operit2 cli workspace exec "$WORKSPACE_ID" git status
operit2 cli workspace delete --yes "$WORKSPACE_ID"
```

`list` returns a `workspaces` array of complete records and a `chatCounts` map
keyed by ID. `create`, `show`, `mount`, `unmount`, and `primary` return
`{ "workspaceId": "...", "workspace": { ... } }`.

`commands` and `run` read `.operit/config.json` from the primary folder. Tool
shortcuts execute with the selected workspace's primary directory and complete
folder list. `exec` runs its argument vector in the primary folder. These
execution commands require a primary folder owned by the current node; they
report an error for a foreign primary folder. File operations in chats retain
the existing owning-node routing.

Deleting uses the shared workspace lifecycle: it clears chat bindings and
removes unshared app-owned workspace files. Mounted host directory contents
are preserved. `summary` and `delete-unbound` remain storage maintenance
commands for app-owned directories, addressed by storage directory name.

## Changes from the previous commands

- `bind <chat-id> <directory>` becomes `mount <workspace-id> <folder-name>
  --native <directory>` followed by `bind <chat-id> <workspace-id>`.
- `default-path`, `create-default`, and `bind-default` are replaced by
  `create`; use its returned ID and folder metadata.
- `commands <chat-id>` and `run <chat-id> <command-id>` now take a workspace ID.
- `commands-path`, `run-path`, and `exec <directory> ...` are replaced by
  workspace-ID operations after mounting the directory.

Retired commands and invalid arguments return an error rather than a successful
usage response. No directory-existence heuristic changes a native input into
a virtual input or a virtual input into a native one.
