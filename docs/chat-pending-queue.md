# Pending message queue attachments

Each `PendingQueueMessageItem` owns the attachment records captured with its
draft. Automatic dequeue, manual send, and queue restoration carry those records
with the text. Queue tiles display their attachment count. Older serialized
items without `attachments` decode with an empty list.

`ChatServiceCore::enqueuePendingQueueMessage` inserts the item and removes its
unchanged submitted attachments from the selected composer's list in one
mutable Core call. A newer attachment remains in the composer; a replacement
with different metadata, VFS locator, or owning node also remains. Enqueueing for
a chat that is no longer selected does not mutate the selected composer's list.

Flutter refreshes the attachment display after Core accepts the queue item. It
does not issue a second clear request. A display refresh that finishes after a
chat switch is discarded. Queued sends use the item's captured attachments and
do not read or clear the current composer.

Regression coverage lives in `ChatServiceCore::pending_queue_attachment_tests`
and `apps/flutter/app/test/chat_pending_queue_attachments_test.dart`. It covers
queue/restore transport, new picks before and after enqueue, another selected
chat, replacement metadata, text-only enqueue, and the queued-send boundary.

The enqueue route now includes `attachments`, so the peer protocol ledger
appends revision 28 after the published revision 27. The existing revision and
its fingerprint remain unchanged.

Focused verification:

```text
cargo test --manifest-path core/Cargo.toml -p operit-runtime --lib pending_queue_attachment_tests -- --test-threads=1
cargo test --manifest-path core/Cargo.toml -p operit-model --lib PendingQueueMessageItem
python tools/check_peer_protocol.py --base origin/main
```

From `apps/flutter/app` with the pinned FVM SDK:

```text
fvm flutter test --no-pub test/chat_pending_queue_attachments_test.dart test/chat_composer_send_action_menu_test.dart
fvm dart analyze lib/ui/features/chat/screens/AIChatScreen.dart lib/ui/features/chat/viewmodel/ChatViewModel.dart test/chat_pending_queue_attachments_test.dart
```
