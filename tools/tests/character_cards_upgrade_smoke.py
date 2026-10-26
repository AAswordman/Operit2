#!/usr/bin/env python3
"""Upgrade a released +15/v27 fixture using an already built macOS Core, without compiling.

The schema comes from the production v27 migration fixture; preference keys and
string encodings match the released cf7c7739 CharacterCardManager writer. No
unreleased plugin database or compatibility metadata is seeded into the fixture.
"""
import argparse
import ctypes
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile

REPO = Path(__file__).resolve().parents[2]
V27_SCHEMA = REPO / "core/crates/persistence/store/src/db/fixtures/version27-chat-records.sql"
EXPECTED_BINDINGS = {
    "v27-chat": "group:legacy-role-group",
    "release15-card-chat": "card:release15-card",
    "release15-default-chat": "card:default_character",
    "release15-null-chat": "card:default_character",
}
HISTORY_TABLES = ("chats", "messages", "message_variants", "message_parts")


def write_fixture(root):
    """Seed only released Core files, with all values encoded by the old writers."""
    runtime = root / "runtime"
    database = runtime / "data/database/operit2.sqlite"
    database.parent.mkdir(parents=True)
    (root / "workspaces").mkdir()
    with sqlite3.connect(database) as db:
        db.executescript(V27_SCHEMA.read_text(encoding="utf-8"))
        for identity, name in [("release15-card-chat", "legacy-card"),
                               ("release15-default-chat", "Operit"),
                               ("release15-null-chat", None)]:
            db.execute("INSERT INTO chats(id,title,createdAt,updatedAt,characterCardName) VALUES(?,?,100,200,?)",
                       (identity, identity, name))
        history = history_rows(db)
    preferences = runtime / "config/preferences"
    preferences.mkdir(parents=True)
    cards = {"__operit_preferences_schema_version": "1",
             "character_card_list": json.dumps(["default_character", "release15-card"]),
             "active_character_card_id": "release15-card"}
    for identity, name in [("default_character", "Operit"), ("release15-card", "legacy-card")]:
        values = {
            "name": name, "description": "Released role", "character_setting": "Keep released prompt",
            "opening_statement": "Hello", "other_content_chat": "Keep chat", "other_content_voice": "Keep voice",
            "advanced_custom_prompt": "Keep advanced prompt", "marks": "", "chat_model_binding_mode": "FOLLOW_GLOBAL",
            "memory_binding_mode": "CHARACTER", "attached_tag_ids": "[]", "shared_memory_mounts": "[]",
            "is_default": str(identity == "default_character").lower(), "created_at": "100", "updated_at": "200",
        }
        cards.update({f"character_card_{identity}_{key}": value for key, value in values.items()})
        memory = runtime / "data/memory/characters" / identity
        memory.mkdir(parents=True)
        (memory / "USER.md").write_text(f"# Keep released USER: {identity}\n", encoding="utf-8")
    (preferences / "character_cards.preferences.json").write_text(json.dumps(cards), encoding="utf-8")
    group = {"id": "legacy-role-group", "name": "Released group", "description": "Keep group",
             "members": [{"characterCardId": "release15-card", "orderIndex": 0}], "createdAt": 100, "updatedAt": 200}
    groups = {"__operit_preferences_schema_version": "1", "character_group_list": json.dumps([group["id"]]),
              "active_character_group_id": "", "character_group_legacy-role-group_data": json.dumps(group)}
    (preferences / "character_groups.preferences.json").write_text(json.dumps(groups), encoding="utf-8")
    return history


def history_rows(db):
    """Compare old history fields exactly, while bindings move out of Core rows."""
    result = {}
    for table in HISTORY_TABLES:
        columns = [row[1] for row in db.execute(f'PRAGMA table_info("{table}")')
                   if row[1] not in ("characterCardName", "characterGroupId", "pluginExtensions")]
        fields = ",".join('"' + column + '"' for column in columns)
        result[table] = db.execute(f'SELECT {fields} FROM "{table}" ORDER BY rowid').fetchall()
    return result


def start_core(app, root):
    """Load the built app's ABI in a separate process with explicit temporary roots."""
    frameworks = app / "Contents/Frameworks"
    libraries = [ctypes.CDLL(str(frameworks / "FlutterMacOS.framework/Versions/A/FlutterMacOS"),
                             mode=ctypes.RTLD_GLOBAL)]
    for framework in sorted(frameworks.glob("*.framework")):
        binary = framework / "Versions/A" / framework.stem
        if framework.stem != "FlutterMacOS" and binary.is_file():
            libraries.append(ctypes.CDLL(str(binary), mode=ctypes.RTLD_GLOBAL))
    library = ctypes.CDLL(str(app / "Contents/MacOS/Operit2.debug.dylib"), mode=ctypes.RTLD_GLOBAL)
    create = library.operit_flutter_bridge_create_with_storage_roots
    create.argtypes, create.restype = [ctypes.c_char_p, ctypes.c_char_p], ctypes.c_void_p
    error = library.operit_flutter_bridge_create_error
    error.restype = ctypes.c_void_p
    free = library.operit_flutter_bridge_free_string
    free.argtypes = [ctypes.c_void_p]
    destroy = library.operit_flutter_bridge_destroy
    destroy.argtypes = [ctypes.c_void_p]
    handle = create(os.fsencode(root / "runtime"), os.fsencode(root / "workspaces"))
    if not handle:
        pointer = error()
        message = ctypes.string_at(pointer).decode("utf-8")
        free(pointer)
        raise AssertionError("Core startup failed: " + message)
    destroy(handle)


def verify(root, before):
    """Require current bindings and preserved history after startup and restart."""
    runtime = root / "runtime"
    candidates = list(runtime.rglob("characters-relational-v2.sqlite"))
    assert len(candidates) == 1, candidates
    with sqlite3.connect(candidates[0]) as db:
        actual = dict(db.execute('SELECT "_key", "selection" FROM "chat_bindings"'))
        assert actual == EXPECTED_BINDINGS, (actual, EXPECTED_BINDINGS)
        names = dict(db.execute('SELECT "_key", "name" FROM "cards"'))
        assert names == {"default_character": "Operit", "release15-card": "legacy-card"}, names
        assert list(db.execute('SELECT "id", "name" FROM "groups"')) == [("legacy-role-group", "Released group")]
        owners = list(db.execute('SELECT "ownerKey", "userDocumentPath" FROM "owners"'))
        assert {owner for owner, _ in owners} == {"character:default_character", "character:release15-card"}, owners
        for owner, relative in owners:
            identity = owner.removeprefix("character:")
            content = (candidates[0].parent / relative).read_text(encoding="utf-8")
            assert content == f"# Keep released USER: {identity}\n", (owner, relative, content)
    with sqlite3.connect(runtime / "data/database/operit2.sqlite") as db:
        assert db.execute("PRAGMA user_version").fetchone()[0] == 28
        assert history_rows(db) == before, "History changed during character-binding adoption"
        for encoded, in db.execute("SELECT pluginExtensions FROM chats"):
            extensions = json.loads(encoded)
            assert "com.operit.character_cards.migration" not in extensions, extensions
            assert "com.operit.character_cards" not in extensions, extensions
    assert not (runtime / "config/preferences/character_cards.preferences.json").exists()
    assert not (runtime / "config/preferences/character_groups.preferences.json").exists()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app-bundle", type=Path, default=REPO / "apps/flutter/app/build/macos/Build/Products/Debug/Operit2.app")
    parser.add_argument("--start-core", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("This smoke test uses the already built macOS Debug app")
    app = args.app_bundle.resolve()
    if args.start_core is not None:
        start_core(app, args.start_core.resolve())
        return
    root = Path(tempfile.mkdtemp(prefix="operit-release15-upgrade-"))
    print("Isolated upgrade fixture:", root, flush=True)
    passed = False
    try:
        before = write_fixture(root)
        for step in ("upgrade", "restart"):
            result = subprocess.run([sys.executable, str(Path(__file__).resolve()), "--app-bundle", str(app),
                                     "--start-core", str(root)], capture_output=True, text=True, timeout=45)
            if result.returncode:
                raise AssertionError(f"{step}: exit={result.returncode}\n{result.stdout}\n{result.stderr}")
            verify(root, before)
            print("PASS", step, "bindings, role records, history, USER files and cleanup", flush=True)
        passed = True
    finally:
        if passed:
            shutil.rmtree(root)
        else:
            print("Failed fixture retained:", root, file=sys.stderr)


if __name__ == "__main__":
    main()
