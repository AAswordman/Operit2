/** Preserves the original root command names while making each plugin parser explicit. */
export type CommandFamily = "character" | "group" | "tag" | "active-prompt" | "memory";

/** Documents the legacy character tree and the complete-record interchange extensions. */
export const CHARACTER_USAGE = [
  "operit2 character list",
  "operit2 character show <id>",
  "operit2 character create <name> [character-setting]",
  "operit2 character create --record <character-json>",
  "operit2 character update <id> <field> <value>",
  "operit2 character update <id> --record <character-json>",
  "operit2 character update <id> --patch <changes-json>",
  "operit2 character delete <id>",
  "operit2 character set-active <id>",
  "operit2 character combine <id> [CHAT|VOICE] [tag-id-csv]",
  "operit2 character reset-default",
  "operit2 character export <id> <operit|tavern>",
  "operit2 character import <operit|tavern> <json-content>",
  "operit2 character export-backup",
  "operit2 character import-backup <json-content>",
];
/** Documents the legacy group tree and native interchange extensions. */
export const GROUP_USAGE = [
  "operit2 group list", "operit2 group show <id>",
  "operit2 group create <name> [description]", "operit2 group create --record <group-json>",
  "operit2 group update <id> <field> <value>", "operit2 group update <id> --record <group-json>", "operit2 group update <id> --patch <changes-json>",
  "operit2 group delete <id>", "operit2 group set-active <id>", "operit2 group duplicate <source-id> [new-name]",
  "operit2 group export <id>", "operit2 group import <json-content>", "operit2 group export-backup", "operit2 group import-backup <json-content>",
];
/** Documents the legacy prompt tag tree and complete-record writes. */
export const TAG_USAGE = [
  "operit2 tag list", "operit2 tag show <id>",
  "operit2 tag create <name> [prompt-content] [description] [tag-type]", "operit2 tag create --record <tag-json>",
  "operit2 tag update <id> <field> <value>", "operit2 tag update <id> --record <tag-json>", "operit2 tag update <id> --patch <changes-json>",
  "operit2 tag delete <id>",
];
/** Documents the active prompt manager's original tree. */
export const ACTIVE_PROMPT_USAGE = [
  "operit2 active-prompt show", "operit2 active-prompt set-card <id>", "operit2 active-prompt set-group <id>",
  "operit2 active-prompt activate-for-chat [character-card-name] [character-group-id]", "operit2 active-prompt resolved-card",
];
/** Documents the memory root without inventing an alternate command name. */
export const MEMORY_USAGE = [
  "operit2 memory <character|shared|mount|unmount>",
  "operit2 memory resolve <character-id>",
];
/** Documents the character memory namespace. */
export const CHARACTER_MEMORY_USAGE = [
  "operit2 memory character <character-id> user <show|write|path>",
  "operit2 memory character <character-id> item <list|search|show|create|update|delete|move>",
  "operit2 memory character <character-id> graph",
  "operit2 memory character <character-id> link <create|update|delete>",
  "operit2 memory character <character-id> settings <show|write>",
  "operit2 memory character <character-id> search-config <show|write>",
  "operit2 memory character <character-id> export",
  "operit2 memory character <character-id> import <SKIP|UPDATE|CREATE_NEW> <json-content>",
];
/** Documents shared library management and its owner-scoped subtrees. */
export const SHARED_MEMORY_USAGE = [
  "operit2 memory shared <list|create|rename|delete>",
  "operit2 memory shared <shared-id> user <show|write|path>",
  "operit2 memory shared <shared-id> item <list|search|show|create|update|delete|move>",
  "operit2 memory shared <shared-id> graph",
  "operit2 memory shared <shared-id> link <create|update|delete>",
  "operit2 memory shared <shared-id> settings <show|write>",
  "operit2 memory shared <shared-id> search-config <show|write>",
  "operit2 memory shared <shared-id> export",
  "operit2 memory shared <shared-id> import <SKIP|UPDATE|CREATE_NEW> <json-content>",
];
/** Documents USER.md operations with the original content argument semantics. */
export const USER_USAGE = [
  "operit2 memory <owner> user show", "operit2 memory <owner> user write <content>", "operit2 memory <owner> user path",
];
/** Documents string item IDs, title lookup, CSV movement, and complete records with quoted JSON identities. */
export const ITEM_USAGE = [
  "operit2 memory <owner> item list", "operit2 memory <owner> item search <query>", "operit2 memory <owner> item show <title>",
  "operit2 memory <owner> item create <title> <content> [folder] [tags-csv]", "operit2 memory <owner> item create --record <memory-json>",
  "operit2 memory <owner> item update <title> <field> <value>", "operit2 memory <owner> item update <title> --patch <changes-json>", "operit2 memory <owner> item update <title> --record <memory-json>",
  "operit2 memory <owner> item delete <id>", "operit2 memory <owner> item move <ids-csv> <folder>",
];
/** Documents string-addressed relationship mutations without routing to the memory tool. */
export const LINK_USAGE = [
  "operit2 memory <owner> link create <source-title> <target-title> <type> <weight> <description>",
  "operit2 memory <owner> link update <link-id> <field> <value>", "operit2 memory <owner> link update <link-id> --patch <changes-json>",
  "operit2 memory <owner> link delete <link-id>",
];
/** Documents explicit permissions required by legacy shared-memory mounts. */
export const MOUNT_USAGE = ["operit2 memory mount <character-id> <shared-id> --read <true|false> --write <true|false>"];
/** Documents the legacy shared-memory unmount operation. */
export const UNMOUNT_USAGE = ["operit2 memory unmount <character-id> <shared-id>"];
