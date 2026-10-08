import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../../../../", import.meta.url));
const backup = "core/crates/runtime/application/src/data/backup/";

/** Reads an exact current source path without replacing an unavailable production implementation. */
function source(relative) { return readFileSync(path.join(root, relative), "utf8"); }

/** Extracts a real Rust function while excluding braces in strings and comments from the source scanner. */
function rustFunction(text, name) {
  const start = text.search(new RegExp("\\b(?:pub(?:\\([^)]*\\))?\\s+)?(?:async\\s+)?fn\\s+" + name + "\\b"));
  assert.notEqual(start, -1, "Missing real Rust function: " + name);
  const masked = text.replace(/r#"[\s\S]*?"#|"(?:\\.|[^"\\])*"|\/\/[^\r\n]*|\/\*[\s\S]*?\*\//g,
    /** Preserves source offsets while removing non-executable delimiters. */
    token => token.replace(/[^\r\n]/g, " "),
  );
  const opening = masked.indexOf("{", start);
  assert.ok(opening > start);
  let depth = 1, end = opening + 1;
  for (; end < masked.length && depth !== 0; end += 1) {
    if (masked[end] === "{") depth += 1;
    if (masked[end] === "}") depth -= 1;
  }
  assert.equal(depth, 0, "Unbalanced source function: " + name);
  return text.slice(start, end);
}

/** Reads actual schema fixture SQL used by the Rust tests rather than inventing a parallel source query. */
function schemaSql(name) {
  const fixtures = source(backup + "operit1/Operit1SnapshotImportTests.rs");
  const match = fixtures.match(new RegExp("const " + name + ": &str = r#\"([\\s\\S]*?)\"#;"));
  assert.ok(match, "Missing source fixture SQL: " + name);
  return match[1];
}

/** Extracts the actual source SELECT and declared reader indexes without executing a fake Rust importer. */
function sourceQuery(body) {
  const sql = body.match(/r#"([\s\S]*?)"#/);
  assert.ok(sql, "The source reader must have a real SQLite query");
  const bindings = [...body.matchAll(/(\w+): sqliteRow(?:OptionalString|String|I64)\(row, (\d+), "chats\.(\w+)"\)/g)]
    .map(([, field, index, column]) => ({ field, index: Number(index), column }));
  assert.ok(bindings.length > 0);
  return { sql: sql[1], bindings };
}

/** Validates declared reader positions against actual SQLite statement columns, including out-of-range failures. */
function assertColumnBindings(columns, bindings) {
  for (const { field, index, column } of bindings) {
    assert.ok(index < columns.length, `${field}: source index ${index} exceeds ${columns.length} query columns`);
    assert.equal(columns[index], column, `${field}: source reader column does not match its query`);
  }
}

for (const version of [10, 20, 21]) {
  /** Executes the actual production SELECT on real SQLite; target Rust projection remains explicitly a source assertion. */
  test(`real SQLite Room ${version} preserves non-null legacy source columns and all reader indexes`, () => {
    const db = new DatabaseSync(":memory:");
    try {
      db.exec(schemaSql("ROOM_V10_SQL"));
      if (version === 20 || version === 21) db.exec(schemaSql("ROOM_V20_SQL"));
      if (version === 21) db.exec("PRAGMA user_version = 21;");
      const name = version === 10 ? "buildChatArchiveFromOperit1RoomV10Database" : "buildChatArchiveFromOperit1RoomV20Database";
      const body = rustFunction(source(backup + "operit1/Operit1ChatMigration.rs"), name);
      const { sql, bindings } = sourceQuery(body);
      const statement = db.prepare(sql);
      statement.setReadBigInts(true);
      statement.setReturnArrays(true);
      const columns = statement.columns().map(column => column.name);
      const rows = statement.all();
      assert.equal(rows.length, 1);
      assert.equal(columns.length, version === 10 ? 13 : 15);
      assertColumnBindings(columns, bindings);
      const row = Object.fromEntries(bindings.map(({ field, index }) => [field, rows[0][index]]));
      assert.equal(row.group, "legacy-folder");
      assert.equal(row.characterCardName, "legacy-card");
      assert.equal(row.locked, 1n);
      assert.equal(row.displayOrder, 14n);
      assert.equal(row.inputTokens, 9223372036854775806n);
      assert.equal(row.parentChatId, "parent-chat");
      if (version === 10) {
        assert.match(body, /characterGroupId: None/);
        assert.match(body, /pinned: false/);
      } else {
        assert.equal(row.characterGroupId, "legacy-group");
        assert.equal(row.pinned, 1n);
      }
      const target = body.slice(body.indexOf("chats.push(OperitArchivedChat {"));
      assert.match(target, /pluginExtensions: std::collections::BTreeMap::new\(\)/);
      assert.doesNotMatch(target, /\b(?:group|characterCardName|characterGroupId)\s*:|default_character|activePrompt/);
      assert.equal(db.prepare("PRAGMA user_version").get().user_version, version);
    } finally { db.close(); }
  });
}

/** Reproduces the removed-column regression as a negative source-scanner fixture, not a production execution claim. */
test("reader contract validation rejects the prior short query and stale locked index", () => {
  assert.throws(() => assertColumnBindings(["id", "title", "locked"], [{ field: "locked", index: 12, column: "locked" }]), /exceeds 3 query columns/);
  assert.throws(() => assertColumnBindings(["id", "title", "locked"], [{ field: "title", index: 2, column: "title" }]), /does not match its query/);
});

/** Checks the actual restore chain adopts supported domains and never calls dormant legacy conversion or domain managers. */
test("full importer keeps model speech preferences chat token and resources with explicit legacy non-adoption", () => {
  const importer = source(backup + "Operit1SnapshotImportManager.rs");
  const restore = rustFunction(importer, "importSnapshotSourceInner");
  let position = restore.indexOf("ParsedOperit1Snapshot::fromSource(source)?");
  assert.ok(position >= 0);
  for (const method of ["importModelConfigFromParsed", "importSpeechPreferences", "importDataStorePreferences", "importChatDatabase", "importTokenStatistics", "importSnapshotFiles"]) {
    const next = restore.indexOf("self." + method + "(");
    assert.ok(next > position, `Restore must preserve the actual ordered call: ${method}`);
    position = next;
    rustFunction(importer, method);
  }
  assert.doesNotMatch(importer, /\b(?:CharacterCardManager|CharacterGroupCardManager|SharedMemoryStoreManager|MemoryRepository)\b|importUserMarkdownPreferences\(|importObjectBoxMemoryStore\(/);
  assert.doesNotMatch(restore, /buildOperit2CharacterCards|buildOperit2CharacterGroups|buildOperit2PromptTags|validateOperit1MemorySpaces|registerApi|ToolPkg/);
  const models = rustFunction(importer, "importModelConfigFromParsed");
  assert.match(models, /ModelConfigManager::new\(self\.paths\.runtime_dir\(\)\.to_path_buf\(\)\)/);
  assert.match(models, /replaceDefaultProviderProfile\(provider\.clone\(\)\)[\s\S]*?\.map_err\([\s\S]*?\)\?/);
  assert.match(models, /FunctionalConfigManager::new/);
  assert.match(models, /setModelForFunction\([\s\S]*?FunctionType::CHAT/);
  const speech = rustFunction(importer, "importSpeechPreferences");
  assert.match(speech, /buildOperit2TtsConfig\(parsed\)\?/);
  assert.match(speech, /importOperit1TtsConfig\(&manager, config\)\?/);
  const chats = rustFunction(importer, "importChatDatabase");
  assert.match(chats, /buildChatArchiveFromOperit1ToOperit2DatabaseBridge/);
  assert.match(chats, /ChatHistoryManager::new\(self\.paths\.clone\(\)\)/);
  assert.match(chats, /importChatArchiveWithProgress/);
  const stats = rustFunction(importer, "importTokenStatistics");
  assert.match(stats, /UsageStatisticsStore::new\(\)/);
  assert.match(stats, /upsertTokenStatsModel\(TokenStatsModel/);
  assert.match(stats, /Operit1RoomV21ToOperit2SqliteV28/);
  const files = rustFunction(importer, "importSnapshotFiles");
  assert.match(files, /buildSnapshotFileCopyPlan\(parsed\)\?/);
  assert.match(files, /\.copyEntriesTo\(&sourceEntries/);
  assert.match(files, /writeArchiveReaderToStorage\([\s\S]*?self\.storageWriteHost\.as_ref\(\)/);
  const inspect = rustFunction(importer, "inspectSnapshotSource");
  assert.doesNotMatch(inspect, /validateOperit1MemorySpaces|buildOperit2Character/);

});

/** Preserves ZIP path security and all source DataStore keys; these assertions do not substitute for running the Rust parser. */
test("archive source format still enumerates and decodes all legacy data without an excluded-domain allowlist", () => {
  const archive = source(backup + "operit1/Operit1SnapshotArchive.rs");
  const reader = rustFunction(archive, "fromSource");
  assert.match(reader, /for index in 0\.\.archive\.len\(\)/);
  assert.match(reader, /validateSnapshotEntryPath\(&name\)\?/);
  assert.match(reader, /datastorePreferences\.insert\(name\.clone\(\), decodeDataStorePreferences\(&bytes\)\?\)/);
  assert.match(reader, /entries\.insert\(name\.clone\(\), entry\)/);
  assert.doesNotMatch(reader, /character_cards|character_groups|prompt_tags|objectbox|memory-space-profiles|isOperit1UnadoptedPreferenceKey/);
  assert.doesNotMatch(archive, /deny_unknown_fields/);
  const parsed = rustFunction(source(backup + "operit1/Operit1ModelMigration.rs"), "fromSource");
  assert.match(parsed, /Operit1SnapshotArchive::fromSource\(source\)\?/);
  assert.doesNotMatch(parsed, /validateOperit1MemorySpaces|buildOperit2Character|isOperit1LegacyMemoryEntry/);
});

/** Ensures mixed mapped preferences are filtered only during restore and counts reflect actually written keys. */
test("datastore restoration excludes legacy adoption while retaining general mappings and truthful counts", () => {
  const preferences = source(backup + "operit1/Operit1PreferenceMigration.rs");
  const mappings = rustFunction(preferences, "datastorePreferenceMappings");
  assert.doesNotMatch(mappings, /character_cards|character_groups|prompt_tags|persona_card_chat_history/);
  for (const entry of ["user_preferences", "api_settings", "display_preferences", "ui_preferences"]) assert.match(mappings, new RegExp(entry + "\\.preferences_pb"));
  const filter = rustFunction(preferences, "isOperit1UnadoptedPreferenceKey");
  for (const key of ["active_prompt_id", "active_character_card_id", "active_memory_space_id", "memory_space_list", "enable_memory_auto_update", "memory_settings"]) assert.ok(filter.indexOf('"' + key + '"') >= 0, "Missing exact excluded source key: " + key);
  assert.doesNotMatch(filter, /\.contains\(/);
  const restore = rustFunction(source(backup + "Operit1SnapshotImportManager.rs"), "importDataStorePreferences");
  assert.match(restore, /adoptedOperit1PreferenceEntries\(preferences, fileImportPlan\)\?/);
  const projection = rustFunction(preferences, "adoptedOperit1PreferenceEntries");
  assert.match(projection, /\.filter\(\|\(key, _\)\| !isOperit1UnadoptedPreferenceKey\(key\)\)/);
  assert.match(projection, /value\.toTargetPreferenceEntry\(key, fileImportPlan\)/);
  assert.doesNotMatch(projection, /unwrap_or|Ok\(Vec::new\(\)\)/);
  assert.match(restore, /keyCount \+= encodedPreferences\.len\(\) as i32/);
  assert.match(restore, /if encodedPreferences\.is_empty\(\) \{\s*continue;/);
});

/** Verifies the actual resource planner excludes declared memory and preference directories without classifying workspace USER.md by name. */
test("file copy plan excludes legacy memory storage but still streams workspace internal and external resources", () => {
  const storage = source(backup + "operit1/Operit1MemoryStorage.rs");
  const exclusion = rustFunction(storage, "isOperit1LegacyMemoryEntry");
  assert.match(exclusion, /entry\.strip_prefix\(ENTRY_FILES_PREFIX\)/);
  assert.match(exclusion, /relative\.split_once\('\/'\)/);
  assert.match(exclusion, /directory == "objectbox"/);
  assert.match(exclusion, /directory\.starts_with\("objectbox_"\)/);
  assert.match(exclusion, /directory == "memory-space-profiles"/);
  assert.doesNotMatch(exclusion, /USER\.md|user\.md|\.contains\(/);
  const selected = rustFunction(storage, "entryMatchesCopyPrefix");
  assert.match(selected, /!entry\.starts_with\(ENTRY_DATASTORE_PREFIX\)/);
  assert.match(selected, /!isOperit1LegacyMemoryEntry\(entry\)/);
  const plan = rustFunction(storage, "buildSnapshotFileCopyPlan");
  assert.match(plan, /entry\.strip_prefix\(ENTRY_WORKSPACE_FILES_PREFIX\)/);
  for (const prefix of ["ENTRY_FILES_PREFIX", "ENTRY_EXTERNAL_FILES_PREFIX"]) assert.match(plan, new RegExp("entryMatchesCopyPrefix\\(entry, " + prefix + "\\)"));
});

/** Keeps the explicit target bridge strict and up to date without rewriting or rejecting the supported original Room source versions. */
test("Room 10 20 and 21 bridge targets match current SQLite 28 without source-schema mutation", () => {
  const schema = source(backup + "Operit1RoomSchemaMigration.rs");
  const version = source("core/crates/persistence/store/src/db/AppDatabase.rs").match(/pub const DATABASE_VERSION: i32 = (\d+);/);
  assert.ok(version);
  assert.equal(Number(version[1]), 28);
  for (const room of [10, 20, 21]) {
    assert.match(schema, new RegExp(`Self::Operit1RoomV${room}ToOperit2SqliteV28 => 28`));
    assert.match(schema, new RegExp(`${room} => Operit1RoomSchemaVersion::V${room}`));
  }
  const select = rustFunction(schema, "selectOperit1ToOperit2ChatArchiveBridge");
  assert.match(select, /targetSchemaVersion != OPERIT2_SQLITE_CURRENT_SCHEMA_VERSION/);
  assert.match(select, /return Err/);
  assert.doesNotMatch(select, /execute|PRAGMA user_version\s*=/);
  for (const file of ["Operit1RoomSchemaMigration.rs", "Operit1SnapshotImportManager.rs", "operit1/Operit1ChatMigration.rs"]) assert.doesNotMatch(source(backup + file), /SqliteV27/);
});

/** Differentiates zero adopted-memory counters and unassociated targets from nonexistent source memories or recovered historical role/TTS state. */
test("preview reports present legacy domains and restore does not claim legacy memory recovery", () => {
  const importer = source(backup + "Operit1SnapshotImportManager.rs");
  const restore = rustFunction(importer, "importSnapshotSourceInner");
  assert.match(restore, /importedMemories: 0/);
  assert.match(restore, /importedMemoryLinks: 0/);
  assert.match(restore, /"unadopted_legacy_domains"/);
  assert.doesNotMatch(importer, /快照没有可迁移的记忆库|正在写入角色卡/);
  const preview = rustFunction(source(backup + "operit1/Operit1ModelMigration.rs"), "preview");
  assert.match(preview, /self\.archive\.datastorePreferences\.keys\(\)/);
  assert.match(preview, /isOperit1LegacyMemoryEntry\(entry\)/);
  assert.match(preview, /detectedDomains\.push\("memory"\.to_string\(\)\)/);
  const messages = source(backup + "operit1/Operit1ChatMigration.rs");
  for (const name of ["readOperit1RoomV10Messages", "readOperit1RoomV20Messages", "readOperit1RoomV20MessageVariants"]) assert.match(rustFunction(messages, name), /pluginExtensions: std::collections::BTreeMap::new\(\)/);
});
