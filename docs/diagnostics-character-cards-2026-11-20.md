# Character-card group planning and missing avatar diagnostics

## Supplied evidence

`operit-diagnostics-1795172584571.log.txt`, exported at 2026-11-20 19:03, records
three group failures at 18:36:29, 18:59:14 and 18:59:49:

```text
[character_cards] 群聊失败 ... Unexpected token '<', "<think>用户现"... is not valid JSON
```

The group hook admitted and saved each user message before planning. The model
requests completed with HTTP 200, and the failure occurred before participant
replies began. The functional model result exposed thinking markup as assistant
text; the group's strict `JSON.parse` correctly rejected that non-JSON payload.

At 19:01:00 and 19:01:03, Flutter failed to read the referenced image:

```text
/operitapp/data/extensions/device/plugins/configs/com.operit.character_cards/avatars/1794970570027-43ruhmevkkq.image
File does not exist
```

The editor stored selected avatars in installation configuration while character
records used shared plugin data. For a device installation, those image files are
device-owned and do not follow shared records. The missing file is confirmed;
the log does not establish whether synchronization, restoration, deletion or
another event caused this particular file to become unavailable.

## Changes

The common functional-model response parser removes thinking and all metadata
blocks before interpreting visible assistant text and tool calls. This preserves
the group's strict JSON validation and provider metadata in its separate result
field. No alternate plan or automatic model retry is added.

The editor writes selected avatars into shared plugin data through the existing
VFS Files API. Startup migration copies referenced avatars from the current
installation's legacy config directory before updating their saved references.
It keeps originals, verifies preexisting destination bytes, preserves independent
character edits and rejects concurrent avatar changes. Missing sources remain
explicitly unresolved and do not block the rest of the role service.

## Verification boundaries

Node regressions exercise the production editor, repository startup and migration
over real binary files and JSON preferences. They cover missing originals, copy
failures, interrupted reference publication, target conflicts and concurrent edits.
Strict no-emit checks cover host/browser TypeScript and existing architecture and
routing contracts. Native parser tests cover thinking, metadata, nested/unfinished
blocks, Unicode offsets and real versus hidden tool calls.

The selected Node run completed 104 cases: 96 passed and eight group-hook cases
stopped because their existing console stub omits the host's `console.verbose`
method. Running that group suite with the method supplied only in the Node process
passed all nine cases:

```powershell
node --input-type=module -e "console.verbose = () => {}; await import('./tests/group-input-hooks.test.mjs');"
```

All seven new avatar migration cases and the actual editor avatar-selection cases
passed directly. `node scripts/check.mjs` passed host/browser strict checks,
architecture checks and routing checks. Non-writing `rustfmt --emit stdout`
parser checks passed for both changed native files.

Rust and Flutter compilation and native tests are not run for this change. The
native edits receive parser checks only. Node file tests do not establish delivery
between two real devices or recovery of the user's already missing image bytes.
The changes require a rebuilt application; no package is deployed by this task.
