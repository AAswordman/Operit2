# Character and group selection and portraits

The character plugin exposes character cards, role groups and model-only mode in
the chat input menu's shared selector. The current group remains selected when
opening the menu. Quick switching uses the selected card or group's most recently
updated conversation, or creates a conversation explicitly bound to that selection.

Both card and group management rows offer batch conversation binding. The target
picker lists cards and groups with distinct `card:` and `group:` identities. Chats
already bound to the target cannot be selected again. Changing targets preserves
eligible selections, and submitting retains individual failed writes for explicit
retry. Batch binding does not change the global active selection.

## Group portraits

`Group.avatarUri` is an optional nullable custom-image reference. Existing stored
groups and backups without that field retain automatic member composition. New
groups save `null` for automatic composition. API creation, updates, duplication,
JSON imports and backups preserve custom references through the ordinary group
record. Image selection copies bytes into the plugin's shared VFS data directory,
using the same picker and storage contract as character cards.

Automatic portraits take the first nine members sorted by `orderIndex`. One member
fills the portrait. Two to four members use two columns, and five to nine use three.
Partial first rows and the complete grid are centered. Each member image is cropped
to a square; members without a custom image use the packaged character portrait.
An empty draft displays the group icon until members are selected.

The editor renders the current draft, so adding and removing members changes the
portrait immediately. Choosing a custom image keeps that image independent of
membership edits. “使用成员拼图” explicitly clears the custom reference. Cancelling
the editor leaves both the saved reference and membership unchanged.

Native previews, management, the selector, the input menu and both sidebar views
share the same composition renderer and layout. Generic chat identity consumers
receive a derived PNG data URI through the existing image APIs. Generated images
are not stored as group configuration or synchronized as additional files. A
bounded in-memory cache shares pending image work; member ordering, image URI and
revision changes produce a new cache identity. Image failures preserve their
original errors, and image handles are released on success and failure.

The generic Jimp image facade adds `cover(width, height)`, returning a separately
owned image with a centered crop. Its implementation uses the existing shared
Rust image service, with output dimensions restricted to 1–4096 pixels per axis.
No platform-specific application branch is required.

## Verification

The Node suites exercise registered menu navigation, group and card batch writes,
new and existing group drafts, custom-image file persistence, cancellation, reset
to member composition, released records, all nine grid sizes, and the bitmap ABI's
composition and handle lifetime. The bitmap ABI tests use an explicitly controlled
host and do not establish native pixel output. A Rust test covers exact crop
dimensions, center pixels, source ownership and invalid dimensions; it is included
for native test runs. Host and browser TypeScript, architecture and routing checks
run without emitting packages. Native compilation and device checks are separate.
