# Group chat native allocation failure

## Evidence and diagnosis

The supplied `Operit2群聊崩溃_提交附件.tar.gz` contains Android crash buffers and application logs, but no exported conversation or role records. Its analysis is supporting evidence, not an implementation specification.

The crash buffer reports repeated allocator failures on `operit-blk-*`. The common sixteen-frame tail is the Host task scheduler and Tokio `LocalSet`, not a unique plugin business operation. A failed 16 KiB `mprotect` does not establish that the process has no contiguous virtual address hole.

The local Android symbol library was rebuilt after the affected release, so its addresses cannot directly symbolize the supplied offsets. Comparing instruction windows from the local Preview.37 APK with the newer unstripped library identifies these unchanged code paths:

| Supplied/Preview.37 PC | Corresponding newer PC | Symbol |
| --- | --- | --- |
| `0x7c4d4c0` | `0x7c9ee04` | `FileBindingService::generateUnifiedDiff` |
| `0x84c6bdc` | `0x84ad84c` | `FileSystemToolExecutor::invokeAndStreamAsync` closure |
| `0x84bf248` | `0x84a5ec0` | outer filesystem execution closure |
| `0x49f9320` | `0x4a3f3d8` | Tokio `LocalSet::run_until` |
| `0x4a223e4` | `0x4a46538` | Host `TrackedTask::poll` |

For `generateUnifiedDiff`, 53 overlapping sixteen-byte windows in the 256-byte neighborhood match at the same displacement, including the allocation call site. This is instruction-based identification, not an assertion that the two complete binaries or their build IDs match.

`StandardFileSystemTools::writeFile` writes the content and then generates a diff between the entire old and new files. The old line diff allocated `(old_line_count + 1) × (new_line_count + 1)` `usize` cells. Updating a 25,000-line file allocated approximately 5 GB on a 64-bit host, independently of the number of changed lines. Group plugins writing sizeable state files reach this shared filesystem path. The inline token diff contained the same allocation pattern.

## Changes

- Use the `similar` crate's exact Myers algorithm for both line edits and inline token edits. Its working space is `O(N + M)`; it uses no deadline or approximate result.
- Preserve unified hunk generation, context lines, UTF-8 byte offsets and the existing minimum similarity requirement for inline highlighting.
- Return immediately for byte-identical files.
- Remove the message-to-view notification detour. Message changes already publish their message flows; view hooks carry only view metadata. Keep open/close, title and workspace notifications on their existing paths, without adding another queue, registry or lock.
- Keep completed stored replies when refreshing history. The CLI reproduction exposed a separate stale-view bug: an earlier delegate's unfinished stream replaced the completed first role reply in the display projection. Its database record was already correct. Completed persisted records now retire that stale stream.

## Reproduction and validation

Provider unit tests exercise a 25,000-line edit, long inline token sequences, Unicode byte spans, duplicate lines, empty files and CRLF normalization. Runtime unit tests verify view replay and authoritative completion during history refresh.

`chat show` returns the current display window. The test uses the existing `get_chat_messages_range` tool to verify persisted content twenty records at a time, and reads only completion/author metadata in matching bounded pages from its isolated SQLite database in read-only mode. It adds no production full-history API and does not change demand loading.

`tools/tests/group_chat_cli_smoke.py` runs a disposable CLI owner and:

1. Calls the real `write_file` tool twice on a 25,000-line file and checks the returned diff.
2. Creates two saved roles with distinct model bindings and a real bound group conversation.
3. Sends ordinary user input through the plugin's actual submit hook.
4. Uses loopback HTTP/SSE model responses to exercise planning and ordered multi-round replies.
5. Checks previous-reply context, exactly one persisted user message, completed persisted assistant messages, author identities and actual group-controller completion before the next submission.

Run from the repository root using the project virtual environment:

```powershell
.venv/Scripts/python.exe tools/tests/group_chat_cli_smoke.py --submissions 8 --rounds 5
```

Verified on 2026-11-16 after removing the full-history API: 25,000-line real tool writes, eight consecutive submissions, eighty completed assistant replies and two context summaries. The test made 24 existing CLI range-tool requests, each covering at most twenty records; the largest default display window observed contained twenty-one records. Completion and author assertions passed for every persisted assistant reply. Artifacts: `C:\Users\12809\AppData\Local\Temp\operit-group-cli-of4h6eir`.

The script retains the isolated profile, CLI transcript, model requests and bounded history-page artifacts, and prints their location. This validates the production CLI, plugin and provider pipelines against a controlled model endpoint. Android allocator behavior and Flutter/GPU rendering still require device validation; this test does not reproduce the original user's unavailable conversation data.
