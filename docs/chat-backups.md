# Daily chat database backups and recovery

Clients select external backup folders with the existing external workspace
picker (`OperitFolderAccess.pickWorkspaceDirectory`) and register them through
`FileSelectionService.mountDirectory`. Backup logic uses that VFS mount on every
platform. Generic picker adapters and filesystem Hosts own platform differences;
there are no backup-specific third-party APIs or Flutter platform branches.

Onboarding includes backup directory selection. Existing users see **选择你的备份目录**
when no folder is configured, the folder disappears, or authorization is revoked.
Data settings can change the directory and create a manual backup. **稍后设置**
defers the prompt for this process only.

## External directory lifetime

The machine-local bootstrap field `chatBackupDirectory` contains only the VFS
mount path and its display label. It is independent of installation, runtime and
workspace roots, and is not synchronized. Old bootstrap records remain readable.
Changing identities or roots requires reselecting the folder when its mount is
absent from the active catalog.

Users must select storage outside the removable application container, such as
an independent iCloud Drive folder on iOS. External snapshot files remain when the
application is uninstalled. After reinstalling, users reselect the original folder
to restore access; application-local mount records and grants do not survive uninstall.
Reselection does not move or delete earlier files. The service rejects live data
and workspace roots, checks existence and probes write/read/delete access, and never
recreates a missing selected directory.

Directory availability matches the shared external workspace mount capability.
In this checkout Web's generic picker returns no selection and OpenHarmony's picker
is unimplemented. Those shared capabilities still need implementation; the backup
feature does not provide a separate platform-specific storage path.

## Native SQLite images

Each `.sqlite` file is a standalone **SQLite format 3** database. No chat archive,
JSON export/import, or model conversion is used to create it. The generic Host
`RuntimeSqliteConnection.snapshotDatabase` captures committed database pages:
native Hosts use SQLite's online backup API, including committed WAL contents;
the browser SQLite Host exports its in-memory database image.

`ChatDatabaseSnapshot` works on a private copy, retaining records in `chats`,
`messages`, `message_variants`, and `message_parts`. Other tables retain their
schema for normal database migrations but contain no records. Workspace bindings
are cleared. `secure_delete` and `VACUUM` remove deleted non-chat bytes. An
application marker and `_operit_chat_backup` metadata record identify the snapshot
and its creation time. Attachment references remain in messages; attachment bytes,
workspace definitions, models, characters, memories and settings are excluded.

Core validates the application marker, supported schema version, integrity,
foreign-key relationships, and metadata, and rejects executable schema objects.
Publication copies through VFS into an external temporary file, compares streaming
SHA-256 digests, publishes under a unique filename, and verifies the published
bytes. Existing names are never overwritten. No database content passes through
Flutter. A matching daily filename counts as success only after SQLite validation.

## Recovery

Data settings has a **恢复聊天记录** section. Onboarding adds **从以前的备份恢复**
after Quick start, Operit1 import and Join device space. Both use the same folder
selection and snapshot list with date, conversation count, message count and filename.
Files from previous installation identities are included. Invalid snapshots are
reported and excluded from the usable list.

Recovery explicitly confirms replacement of all current chats. Before replacement,
the service saves current chats as a separate `before-restore` snapshot; failure
stops recovery. An empty current database needs no safety snapshot. Automatic backup
checks pause while recovery runs. Active conversation execution blocks replacement.

Core verifies an isolated staged copy, applies normal schema migrations, then uses
the existing SQL synchronization correction transaction to replace chat tables
atomically and seal covered prior operations. Old synchronization replay cannot undo
the restored state. Other application data stays current. The current-chat selection
is cleared and normal store invalidation refreshes conversation views. A failed
replacement rolls back the transaction; the external source snapshot remains intact.
Recovered chats without execution ownership are explicitly initialized on the
current device; existing routing bindings are preserved and never read from backups.
Onboarding proceeds to permission and model configuration after recovery, since
chat backups do not include provider settings.

## Scheduling and retention

Checks run after onboarding, on foreground resume, and every five minutes while
the main application is active. Closed or suspended applications do not execute a
scheduled OS background job. The next foreground check saves the current state;
it cannot reconstruct a missed day's state.

One verified automatic snapshot per local calendar day uses
`operit-chats-<identity>-<date>-<timestamp>.sqlite`. Empty databases create no snapshot
and prune nothing. After successful publication, the latest ten automatic snapshots
of that identity remain. Manual and `before-restore` files are outside automatic
retention. A fresh installation cannot prune the previous identity's backups.

## Validation

Tests cover WAL capture, the native SQLite header, message revisions, removal of
non-chat bytes, restoration with current unrelated data preserved, atomic rollback,
active-generation rejection, old-operation replay, invalid/newer database rejection,
external VFS publication and readback, missing-directory handling, daily deduplication,
retention ownership, safety backup failures, previous-installation listings, and
the recovery confirmation UI. Device checks must still cover process restarts,
provider permission persistence and actual uninstall/reinstall on each supported Host.
