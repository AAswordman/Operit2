# Daily database file backups and recovery

Clients select an external backup folder with the existing workspace picker and
register it through `FileSelectionService.mountDirectory`. All backup I/O uses
that VFS mount and the existing filesystem Host on every platform.

## File copying

A backup is a direct copy of the complete application SQLite database file.
The shared store closes its SQLite connection, copies the file through VFS, and
reopens the connection. Closing SQLite writes committed WAL contents back to the
main database before copying. Other users of the same store wait on its connection
mutex during the copy. Browser SQLite already persists its database image after
writes; the same close/copy/open sequence uses the browser filesystem Host.

The backup copy does not export chat tables, filter records, run VACUUM, check
integrity or foreign keys, hash contents, stage a database, or read the copy back.
All tables and their contents remain as they are in the source database. Separate
preferences, attachment files, workspaces and other external files are not copied.
The filename is `operit-chats-<identity>-<date>-<timestamp>.sqlite`; manual copies
include `manual-` before the date. Existing backup filenames are not overwritten.
A failed backup copy removes its incomplete destination.

## Recovery

The restore list reads filenames and their embedded timestamps only. It never
opens, copies, validates or counts records in a backup to display the list.
Files from previous installations and older `before-restore` filenames are listed.

After confirmation, the service copies the selected file beside the live database.
The store closes its connection, renames that sibling over the database, and reopens
it. Publication uses the local Host's same-filesystem rename, so a failed source copy
does not truncate the live database. The sibling is not another data conversion or
validation pass. Normal schema migrations and store invalidation run afterward.
The same shared store and its DAO readers keep using the reopened connection. Running conversations must
finish before replacement. The source backup remains in the external folder.
Recovery replaces every table in the database, including synchronization state
and usage data. It does not create a safety backup or restore records through the
synchronization correction transaction.

## Restored synchronization baseline

After replacement, `SqlChatSyncStore.establishRestoreBaseline` writes only a new
authority marker and resets conflict metadata. It does not scan conversation rows,
serialize records, compare contents or build a sync snapshot during restoration.
The next synchronization export materializes the initializer once through the
existing sync mechanism. A persistent restore lineage is carried in that initializer,
its metadata and consistency manifests.
This explicit lineage outranks ordinary content size and business revisions, so a
smaller restored database can correct a larger peer.

The restore event has its own covered receipt. A peer must receive its initializer
even when it already knows every historical operation from the backup. Previously
retained and subsequently redelivered operations from an earlier lineage are sealed
without being applied. New operations use a fresh origin derived from the current
restore lineage; rolling database sequence counters back cannot reuse an old
operation identity. Peers that have adopted the baseline can exchange new edits
normally. Older initializers cannot replace a newer restore lineage.

Consistency protocol version 9 requires both endpoints to understand this authority
rule. Incompatible endpoints are refused before operation exchange.

## Scheduling and retention

Checks run after onboarding, on foreground resume, and every five minutes while
the main application is active. An existing filename for the current identity and
local date completes the daily check without reading database contents. Otherwise
the service copies the database once. After that copy succeeds, the latest ten
automatic files of the current identity remain. Manual backups and files belonging
to previous identities are outside automatic retention. Empty databases are copied
in the same way as databases containing chats.

## External directory lifetime

The machine-local bootstrap field `chatBackupDirectory` stores the mount path and
its display label. Users select storage outside the removable app container, such
as an independent iCloud Drive folder on iOS. After reinstalling, they select the
original external folder again to regain access. Missing folders are reported
without being recreated. Accessibility checks list the folder, write a uniquely
named `.operit-backup-probe-*` file, read it back to verify its contents, and delete
it. All steps must succeed before saving a selected destination. Cleanup is also
attempted for a probe left by a failed write or read; a cleanup failure rejects
the directory. These checks run on selection, startup, foreground resume and every
five minutes while the application is active, even when today's backup exists.
Actual copy failures report filesystem errors and recheck directory usability.
Directory selection and access failures show guidance in onboarding and data
settings to create and select a new directory inside an accessible folder, such
as Documents or Downloads. Failed selection keeps the previous backup destination.

Directory selection uses the shared external workspace mount capability. In this
checkout Web's generic picker returns no selection and OpenHarmony's picker is
unimplemented; those shared capabilities still need implementation.

## Checks

Coverage includes direct-file capture of committed WAL rows, preservation of all
database tables and message variants, reopened shared readers, access after a
failed copy, filename-only daily checks and listings, restoration without an extra
backup, retention ownership, missing-directory handling, restored baseline priority
over larger peers, retirement of old history, and new edits after baseline adoption.
Device I/O throughput and external provider behavior require device verification.
