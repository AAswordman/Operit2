"""Run current Rust startup and plugin migration on a verified copy of a restored Windows profile."""
from __future__ import annotations

import argparse
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
from urllib.parse import quote
from uuid import UUID

REPOSITORY = Path(__file__).resolve().parents[2]
PLUGIN = "com.operit.character_cards"


def read_json(path: Path):
    """Read an explicit UTF-8 JSON artifact without changing its source."""
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value):
    """Write a reviewable artifact inside the caller's disposable acceptance directory."""
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def digest(path: Path) -> str:
    """Hash complete original bytes without decoding user content."""
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inventory(root: Path) -> dict[str, str]:
    """Record every regular file and reject links that could escape the selected profile."""
    result = {}
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise AssertionError("Acceptance source contains a symbolic link: " + str(path))
        if path.is_file():
            result[path.relative_to(root).as_posix()] = digest(path)
    return result


class Credential(ctypes.Structure):
    """Describe the read-only Windows credential structure used by the existing native host."""
    _fields_ = [("flags", wintypes.DWORD), ("kind", wintypes.DWORD),
                ("target", wintypes.LPWSTR), ("comment", wintypes.LPWSTR),
                ("written", wintypes.FILETIME), ("size", wintypes.DWORD),
                ("blob", ctypes.POINTER(ctypes.c_ubyte)), ("persist", wintypes.DWORD),
                ("attribute_count", wintypes.DWORD), ("attributes", ctypes.c_void_p),
                ("alias", wintypes.LPWSTR), ("username", wintypes.LPWSTR)]


def copy_encryption_secret(destination: Path):
    """Copy the existing encryption key into isolated file secrets without updating Windows credentials."""
    library = ctypes.WinDLL("Advapi32.dll", use_last_error=True)
    pointer = ctypes.POINTER(Credential)()
    library.CredReadW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
                                 ctypes.POINTER(ctypes.POINTER(Credential))]
    library.CredReadW.restype = wintypes.BOOL
    library.CredFree.argtypes = [ctypes.c_void_p]
    key = "operit.preferences.encryption_key.v1"
    if not library.CredReadW("Operit2:" + key, 1, 0, ctypes.byref(pointer)):
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        content = ctypes.string_at(pointer.contents.blob, pointer.contents.size)
        destination.mkdir(parents=True, exist_ok=False)
        (destination / key).write_bytes(content)
    finally:
        library.CredFree(pointer)


def readonly(path: Path):
    """Open an explicit acceptance copy in SQLite read-only mode."""
    return sqlite3.connect(path.as_uri() + "?mode=ro", uri=True)


def core_snapshot(path: Path) -> dict:
    """Capture complete chat content and immutable row identities before and after upgrade."""
    with readonly(path) as database:
        result = {"version": database.execute("PRAGMA user_version").fetchone()[0], "tables": {}}
        for table in ["chats", "messages", "message_variants", "message_parts"]:
            columns = [row[1] for row in database.execute('PRAGMA table_info("' + table + '")')]
            rows = [dict(zip(columns, row)) for row in database.execute('SELECT * FROM "' + table + '" ORDER BY rowid')]
            result["tables"][table] = rows
        return result


def prepare(recovery: Path, root: Path):
    """Copy the restored identity and its workspaces while retaining a full source hash manifest."""
    source = Path(read_json(recovery / "publication-journal.json")["target"]).resolve(strict=True)
    root = root.resolve()
    assert not root.exists(), "Acceptance directory must be new"
    assert not root.is_relative_to(source) and not source.is_relative_to(root)
    root.mkdir(parents=True)
    baseline = inventory(source)
    identity = source.name
    runtime = root / "runtime" / "identities" / identity
    shutil.copytree(source, runtime)
    workspace_source = source.parents[2] / "workspaces" / "identities" / identity
    shutil.copytree(workspace_source, root / "workspaces" / "identities" / identity)
    assert inventory(runtime) == baseline, "Acceptance copy differs from its restored source"
    before = core_snapshot(runtime / "data/database/operit2.sqlite")
    assert before["version"] == 27, "This acceptance requires the restored v27 input"
    write_json(root / "source-inventory.json", baseline)
    write_json(root / "core-before.json", before)
    shutil.copy2(recovery / "compatibility-record-input.json", root / "expected-records.json")
    shutil.copy2(recovery / "restoration-report.json", root / "restoration-report.json")
    (root / "config").mkdir()
    (root / "appdata").mkdir()
    write_json(root / "config/storage.json", {"runtimeRoot": str(root / "runtime"),
        "workspaceRoot": str(root / "workspaces"), "activeIdentityId": identity,
        "identities": [{"id": identity, "name": "Restored profile migration acceptance", "createdAt": 1}]})
    copy_encryption_secret(runtime / "secrets")
    write_json(root / "acceptance-input.json", {"source": str(source), "identity": identity,
        "recovery": str(recovery), "sourceFileCount": len(baseline), "sourceDatabaseVersion": 27})
    print("PREPARED: verified independent copy of", len(baseline), "restored files", flush=True)


def run_production(root: Path, phase: int):
    """Execute native startup and genuine plugin commands without starting remote peer services."""
    environment = dict(os.environ, RUSTFLAGS="-Awarnings", OPERIT_CLI_CONFIG_DIR=str(root / "config"),
        APPDATA=str(root / "appdata"), OPERIT_CLI_SECRET_STORE="file",
        OPERIT_MIGRATION_ACCEPTANCE_ROOT=str(root),
        OPERIT_MIGRATION_ACCEPTANCE_REPORT=f"commands-{phase}.json")
    command = ["cargo", "test", "--offline", "--manifest-path", "apps/cli/Cargo.toml",
        "restored_profile_migration_acceptance", "--", "--ignored", "--nocapture", "--test-threads=1"]
    print("RUNNING: current production startup and plugin reads, phase", phase, flush=True)
    with (root / f"native-{phase}.log").open("w", encoding="utf-8") as output:
        result = subprocess.run(command, cwd=REPOSITORY, env=environment, stdout=output, stderr=subprocess.STDOUT)
    if result.returncode != 0:
        raise AssertionError(f"Production acceptance failed; inspect {root / f'native-{phase}.log'}")
    print("PASSED: production startup and plugin reads, phase", phase, flush=True)



def uuid_identity(value: str):
    """Require the canonical plugin identity format without modifying external references."""
    assert str(UUID(value)) == value, "Noncanonical plugin UUID"


def unchanged(actual: dict, expected: dict, changed: set[str]):
    """Compare every persisted business field except the explicitly converted identity fields."""
    assert set(actual) == set(expected), "Migration changed record fields"
    assert {k: v for k, v in actual.items() if k not in changed} == {
        k: v for k, v in expected.items() if k not in changed}, "Migration changed business content"


def match_record(records: list[dict], expected: dict, changed: set[str]) -> dict:
    """Resolve one converted record by all immutable fields and reject ambiguous matches."""
    matches = [record for record in records if {k: v for k, v in record.items() if k not in changed}
               == {k: v for k, v in expected.items() if k not in changed}]
    assert len(matches) == 1, "Missing or ambiguous migrated record"
    unchanged(matches[0], expected, changed)
    return matches[0]


def verify_migration(root: Path) -> dict:
    """Prove complete source-to-current content, identity, relationship and resource preservation."""
    manifest = read_json(root / "acceptance-input.json")
    source = Path(manifest["source"])
    runtime = root / "runtime/identities" / manifest["identity"]
    plugin = runtime / "plugin_data/space" / PLUGIN
    expected = read_json(root / "expected-records.json")
    restored = read_json(root / "restoration-report.json")
    before = read_json(root / "core-before.json")
    after = core_snapshot(runtime / "data/database/operit2.sqlite")
    assert after["version"] == 28, "Core SQLite schema was not upgraded"
    for table, rows in before["tables"].items():
        migrated = after["tables"][table]
        assert len(rows) == len(migrated), "Core row count changed: " + table
        for original, current in zip(rows, migrated):
            retired = {"characterCardName", "characterGroupId"} if table == "chats" else set()
            assert {k: current[k] for k in original if k not in retired} == {
                k: v for k, v in original.items() if k not in retired}, "Core content changed: " + table
            assert not retired.intersection(current), "Legacy Core role columns remain"
    with readonly(runtime / "data/database/operit2.sqlite") as database:
        assert database.execute("PRAGMA integrity_check").fetchall() == [("ok",)]
        assert not database.execute("PRAGMA foreign_key_check").fetchall()
        assert not database.execute("SELECT 1 FROM sync_sql_operations WHERE domain='chat' AND schemaVersion!=8").fetchall()
        for table in ["chats", "messages", "message_variants", "sync_sql_chat_rows",
                      "sync_sql_message_rows", "sync_sql_message_variant_rows"]:
            for (encoded,) in database.execute("SELECT pluginExtensions FROM " + table):
                assert PLUGIN + ".migration" not in json.loads(encoded), "Legacy extension was not retired"
    preferences = {file.stem.removesuffix(".preferences"): read_json(file)
                   for file in plugin.glob("*.preferences.json")}
    metadata = preferences["meta"]
    state = metadata["state"]
    assert metadata["__operit_preferences_schema_version"] == 1
    assert state["version"] == 1 and state["migrationStatus"] == "complete"
    assert "legacyImportPending" not in state and "plan" not in preferences["migration"]
    assert metadata["import_operit2-v27"]["complete"] is True
    assert metadata["import_operit2-v27:themes"]["complete"] is True
    cards = [v for k, v in preferences["cards"].items() if k.startswith("record_")]
    assert len(cards) == len(expected["cards"])
    identities = {}
    user_before = read_json(source / "config/preferences/user_preferences.preferences.json")
    user_after = read_json(runtime / "config/preferences/user_preferences.preferences.json")
    for key, value in user_before.items():
        if key.startswith("theme_config_"):
            assert user_after[key] == value, "Existing independent theme was changed"
    for card in expected["cards"]:
        current = match_record(cards, card, {"id", "themeConfigId"})
        uuid_identity(current["id"])
        try:
            UUID(card["id"])
        except ValueError:
            pass  # The released identity is explicitly converted by this acceptance.
        else:
            assert current["id"] == card["id"], "Existing UUID changed"
        identities[card["id"]] = current["id"]
        assert preferences["cards"]["record_" + current["id"]] == current
        if card["themeConfigId"] is None:
            uuid_identity(current["themeConfigId"])
            theme = user_after["theme_config_" + current["themeConfigId"]]
            assert theme["name"] == card["name"]
            assert theme["createdAt"] == card["createdAt"] and theme["updatedAt"] == card["updatedAt"]
        else:
            assert current["themeConfigId"] == card["themeConfigId"]
    old_selection = read_json(source / "config/preferences/character_cards.preferences.json")["active_character_card_id"]
    assert state["active"] == {"CharacterCard": {"id": identities[old_selection]}}
    bindings = {k.removeprefix("binding_"): v for k, v in preferences["chats"].items() if k.startswith("binding_")}
    assert len(bindings) == len(before["tables"]["chats"])
    bound = unbound = 0
    for chat in before["tables"]["chats"]:
        assert not chat["characterGroupId"], "This restored fixture unexpectedly contains a group binding"
        if chat["characterCardName"]:
            matches = [card for card in expected["cards"] if card["name"] == chat["characterCardName"]]
            assert len(matches) == 1, "Legacy chat binding is ambiguous"
            selection = "card:" + identities[matches[0]["id"]]
            bound += 1
        else:
            selection = "model:chat"
            unbound += 1
        assert bindings[chat["id"]] == {"selection": selection, "version": 1}
    assert bound == restored["recoveredChatBindings"] and unbound == restored["deliberatelyUnboundChats"]
    collections = {}
    with readonly(plugin / "memory.sqlite") as database:
        assert database.execute("PRAGMA integrity_check").fetchall() == [("ok",)]
        assert not database.execute("PRAGMA foreign_key_check").fetchall()
        for collection, key, value in database.execute("SELECT collection,key,value FROM _operit_values ORDER BY collection,key"):
            collections.setdefault(collection, {})[key] = json.loads(value)
    assert len(collections["owners"]) == len(expected["owners"])
    counts = {name: 0 for name in ["memories", "links", "chunks", "candidates", "embeddings"]}
    resources = {}
    for owner in expected["owners"]:
        old_owner = owner["ownerKey"]
        assert old_owner.startswith("character:")
        new_owner = "character:" + identities[old_owner.removeprefix("character:")]
        current = collections["owners"][new_owner]
        metadata_fields = {k: v for k, v in owner.items() if k not in counts}
        unchanged(current, metadata_fields, {"ownerKey", "userDocumentPath"})
        assert current["ownerKey"] == new_owner
        assert current["userDocumentPath"] == "owners/" + quote(new_owner, safe="") + "/USER.md"
        document = next(document for document in restored["documents"] if document["owner"] == old_owner)
        path = plugin / current["userDocumentPath"]
        assert path.stat().st_size == document["bytes"] and digest(path) == document["sha256"]
        resources[current["userDocumentPath"]] = digest(path)
        if old_owner != new_owner:
            assert not (plugin / "owners" / quote(old_owner, safe="")).exists(), "Old owner directory remains"
        records = {}
        for collection in counts:
            records[collection] = []
            for key, envelope in collections.get(collection, {}).items():
                if envelope["ownerKey"] != new_owner:
                    continue
                record = envelope["record"]
                assert key == new_owner + "/" + record["id"]
                uuid_identity(record["id"])
                records[collection].append(record)
            assert len(records[collection]) == len(owner[collection]), "Memory collection count changed: " + collection
            counts[collection] += len(records[collection])
        memory_ids = {}
        for memory in owner["memories"]:
            migrated = match_record(records["memories"], memory, {"id", "uuid", "tags", "properties"})
            memory_ids[memory["id"]] = migrated["id"]
            uuid_identity(migrated["uuid"])
            for field in ["tags", "properties"]:
                assert len(memory[field]) == len(migrated[field])
                for old in memory[field]:
                    uuid_identity(match_record(migrated[field], old, {"id"})["id"])
        for link in owner["links"]:
            converted = dict(link, sourceMemoryId=memory_ids[link["sourceMemoryId"]], targetMemoryId=memory_ids[link["targetMemoryId"]])
            match_record(records["links"], converted, {"id"})
        assert not owner["chunks"] and not owner["embeddings"], "Fixture now requires document/embedding reference checks"
        for candidate in owner["candidates"]:
            messages = [message for message in before["tables"]["messages"] if message["chatId"] == candidate["chatId"]
                        and message["timestamp"] <= candidate["triggerMessageTimestamp"]]
            assert candidate["sourceType"] == "reply_finalized_auto" and messages
            trigger = max(messages, key=lambda message: message["timestamp"])
            assert trigger["sender"] in ["ai", "assistant"]
            converted = dict(candidate, triggerMessageTimestamp=trigger["timestamp"], triggerVariantIndex=trigger["selectedVariantIndex"])
            match_record(records["candidates"], converted, {"id"})
    for collection, count in counts.items():
        assert len(collections.get(collection, {})) == count, "Unexpected owner records"
    for path in ["character_cards", "character_groups", "prompt_tags", "shared_memory_stores"]:
        assert not (runtime / "config/preferences" / (path + ".preferences.json")).exists(), "Legacy configuration remains"
    assert not (runtime / "data/memory/characters").exists(), "Legacy memory owner remains"
    summary = {"coreSchema": after["version"], "pluginSchema": 1, "cards": len(cards), "owners": len(expected["owners"]),
               **counts, "chats": len(after["tables"]["chats"]), "messages": len(after["tables"]["messages"]),
               "messageParts": len(after["tables"]["message_parts"]), "roleBindings": bound, "modelBindings": unbound,
               "verifiedUserDocuments": len(resources), "retainedNamedThemes": sum(k.startswith("theme_config_") for k in user_before)}
    return {"summary": summary, "preferences": preferences, "memory": collections, "documents": resources, "core": after}


def stable_command_result(path: Path) -> dict:
    """Compare complete command content while excluding only the documented export creation time."""
    report = read_json(path)
    for exported in report["memoryExports"]:
        content = json.loads(exported["content"])
        assert isinstance(content["exportDate"], int)
        del content["exportDate"]
        exported["content"] = content
    return report

def main():
    """Prepare and retain a complete migration acceptance run for independent review."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--recovery", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--prepared", action="store_true")
    args = parser.parse_args()
    root = args.work_dir.resolve()
    if not args.prepared:
        prepare(args.recovery.resolve(strict=True), root)
    if args.prepare_only:
        return
    try:
        run_production(root, 1)
        first = verify_migration(root)
        write_json(root / "verified-1.json", first)
        print("PASSED: complete content, identities, references and resource verification", flush=True)
        run_production(root, 2)
        second = verify_migration(root)
        write_json(root / "verified-2.json", second)
        assert first == second, "Reopen changed durable migrated state"
        assert stable_command_result(root / "commands-1.json") == stable_command_result(root / "commands-2.json"), "Reopen changed migrated plugin records"
        write_json(root / "acceptance-result.json", {"status": "passed", **second["summary"], "independentStarts": 2})
    finally:
        source = Path(read_json(root / "acceptance-input.json")["source"])
        assert inventory(source) == read_json(root / "source-inventory.json"), "Original restored profile changed"
        print("PASSED: original restored profile remains byte-identical", flush=True)
    print("PASSED: migration, repeated production reads and durable content preservation", flush=True)


if __name__ == "__main__":
    main()
