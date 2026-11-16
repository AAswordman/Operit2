#!/usr/bin/env python3
"""Measure SQL, preference and file backlogs through isolated real CLI processes.

All SQL/preference writes use public business APIs. The replica is stopped while
preparing each backlog; preparation and reconnect-to-convergence are timed apart.
Full preference enumeration runs after drain because that API itself is expensive.
"""
import argparse
import asyncio
import hashlib
import json
import os
import sqlite3
import uuid
from pathlib import Path
import time
import types

from space_stress_session import Run
from storage_sync_session import identity_root, inspect_nodes

DOMAINS = ['preferences', 'chat', 'binding', 'runtime_file', 'network_control', 'plugin_storage']
CHAT = 'chatRuntimeHolderMain'


async def call(node, schema, method, **args):
    return await node.command('call', schema, method, json.dumps(args), timeout=600)


def message(index, content, timestamp):
    return dict(sender='user', parts=[dict(partId='part-0', sequence=0,
        kind='markdown', content=content, toolCallId=None, toolName=None, attributes={})],
        timestamp=timestamp + index, roleName='', pluginExtensions={},
        selectedVariantIndex=0, variantCount=1, provider='', modelName='',
        inputTokens=0, outputTokens=0, cachedInputTokens=0, sentAt=0,
        outputDurationMs=0, waitDurationMs=0, completedAt=0, displayMode='NORMAL',
        isFavorite=False, contentStream=None)


def sql_rows(node, prefix=None):
    database = identity_root(node, 'runtimeRoot') / 'data/database/operit2.sqlite'
    with sqlite3.connect(database.as_uri() + '?mode=ro', uri=True, timeout=1) as connection:
        if prefix is None:
            return connection.execute('SELECT COUNT(*) FROM messages').fetchone()[0]
        return dict(connection.execute(
            "SELECT chatId, content FROM message_parts WHERE chatId LIKE ? AND variantIndex=0 AND kind='markdown'",
            (prefix + '%',)).fetchall())


def archive(chat_id, index, content, stamp):
    # One import is one real business transaction; do not inject SQL into the store.
    chat = dict(id=chat_id, title=f'Backlog {index}', pluginExtensions={},
        messages=[dict(baseMessage=message(index, content, stamp), variants=[])],
        createdAt='2026-11-16T12:00:00', updatedAt='2026-11-16T12:00:00',
        inputTokens=0, outputTokens=0, currentWindowSize=0, displayOrder=index,
        workspaceId=None, parentChatId=None, locked=False, pinned=False)
    return dict(archiveType='operit_chat_archive', formatVersion=5,
                exportedAt=stamp, workspaces=[], chats=[chat])


async def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--sql-operations', type=int, default=1000)
    parser.add_argument('--sql-shape', choices=['distinct', 'replace'], default='distinct',
                        help='Distinct chat imports or repeated replacement of one chat')
    parser.add_argument('--preference-records', type=int, default=300)
    parser.add_argument('--file-count', type=int, default=8)
    parser.add_argument('--file-bytes', type=int, default=1024 * 1024)
    parser.add_argument('--message-bytes', type=int, default=4096)
    parser.add_argument('--scenarios', nargs='+', choices=['sql', 'preference', 'mixed'],
                        default=['sql', 'preference', 'mixed'])
    parser.add_argument('--timeout', type=float, default=600)
    args = parser.parse_args()
    if min(args.sql_operations, args.preference_records,
           args.file_bytes, args.message_bytes) <= 0 or args.file_count < 0 or args.timeout <= 0:
        parser.error('workload sizes and timeout must be positive; file count may be zero')
    run = Run(types.SimpleNamespace(binary=args.binary, transport='tcp', focus='all',
                                   settle_seconds=args.timeout, operations=1))
    results = []
    try:
        a, b = await run.node('A'), await run.node('B')
        await run.pair(b, a)
        await run.join(b, a, a, [a, b])
        await asyncio.gather(a.command('space', 'sync', timeout=600),
                             b.command('space', 'sync', timeout=600))
        for scenario in args.scenarios:
            run.event('prepare-start', scenario=scenario, epoch=time.time())
            receipt = await call(b, 'application', 'syncClock')
            assert not receipt['errors'], receipt['errors']
            await b.stop()
            prepare_started = time.monotonic()
            sql_count = args.sql_operations if scenario in ('sql', 'mixed') else 0
            pref_count = args.preference_records if scenario in ('preference', 'mixed') else 0
            file_count = args.file_count if scenario == 'mixed' else 0
            expected_sql = {}
            sql_prefix = f'backlog-{scenario}-{uuid.uuid4().hex}-'
            if sql_count:
                stamp = int(time.time() * 1000)
                for index in range(sql_count):
                    chat_id = sql_prefix + (str(index) if args.sql_shape == 'distinct' else 'same')
                    text = (f'{scenario}-{index}:' + 's' * args.message_bytes)[:args.message_bytes]
                    imported = await call(a, CHAT, 'importChatHistoriesFromJson',
                        jsonString=json.dumps(archive(chat_id, index, text, stamp)))
                    assert imported['new'] + imported['updated'] == 1, imported
                    expected_sql[chat_id] = text
                    if (index + 1) % 100 == 0:
                        run.event('prepare-progress', scenario=scenario, sql=index + 1)
            expected_preferences = []
            for index in range(pref_count):
                record = await call(a, 'preferences.ttsConfigManager', 'createTtsConfig',
                    config=dict(id='', name=f'Backlog-{scenario}-{index}',
                        providerType='OPENAI_COMPATIBLE', endpoint=f'https://sync-test.invalid/{scenario}/v{index}',
                        apiKey='synthetic-sync-key', model='sync-model', createdAt=0, updatedAt=0,
                        voice=f'voice-{index}', responseFormat='mp3', speed=1.0))
                expected_preferences.append(record)
                if (index + 1) % 100 == 0:
                    run.event('prepare-progress', scenario=scenario, preferences=index + 1)
            files = []
            source_root = identity_root(a, 'workspaceRoot') / 'backlog' / scenario
            target_root = identity_root(b, 'workspaceRoot') / 'backlog' / scenario
            if file_count:
                source_root.mkdir(parents=True, exist_ok=True)
                for index in range(file_count):
                    payload = os.urandom(args.file_bytes)
                    filename = f'{index}.bin'
                    (source_root / filename).write_bytes(payload)
                    files.append((target_root / filename, hashlib.sha256(payload).hexdigest()))
            # Publish outboxes and discover files before timing transport.
            local_clock = await call(a, 'application', 'syncClock')
            assert not local_clock['errors'], local_clock['errors']
            inventory = {}
            for domain in DOMAINS:
                page = await call(a, 'application', 'syncOperationsSince',
                    clock={**receipt, 'includeProgress': True}, domains=[domain], limit=1)
                assert not page['errors'] and not page['progressErrors'], page
                inventory[domain] = page['remaining']
            prepare_seconds = time.monotonic() - prepare_started
            run.event('backlog-ready', scenario=scenario, prepare_seconds=prepare_seconds,
                sql_shape=args.sql_shape,
                sql_api_writes=sql_count, preference_api_creates=pref_count,
                message_payload_bytes=sql_count * args.message_bytes,
                files=file_count, file_bytes=file_count * args.file_bytes,
                missing_operations=inventory, epoch=time.time())
            started = time.monotonic()
            milestones = {}
            await b.start()
            milestones['replica_ready'] = time.monotonic() - started
            async def synchronize():
                await a.command('space', 'sync', timeout=600)
                return time.monotonic() - started
            sync = asyncio.create_task(synchronize())
            deadline = time.monotonic() + args.timeout
            while time.monotonic() < deadline:
                rows = sql_rows(b, sql_prefix)
                counts = len(rows)
                if sql_count and rows and 'sql_first_visible' not in milestones:
                    milestones['sql_first_visible'] = time.monotonic() - started
                if rows == expected_sql:
                    milestones.setdefault('sql_all_visible', time.monotonic() - started)
                complete_files = sum(p.is_file() and hashlib.sha256(p.read_bytes()).hexdigest() == digest
                                     for p, digest in files)
                if file_count and complete_files:
                    milestones.setdefault('file_first_visible', time.monotonic() - started)
                if complete_files == file_count:
                    milestones.setdefault('file_all_visible', time.monotonic() - started)
                if sync.done():
                    # Propagate failures promptly, even if content checks are incomplete.
                    sync.result()
                    if all(key in milestones for key in ('sql_all_visible', 'file_all_visible')):
                        break
                await asyncio.sleep(.1)
            else:
                raise AssertionError(f'{scenario} did not converge: {milestones}, messages={counts}, files={complete_files}')
            assert sql_rows(b, sql_prefix) == expected_sql, 'SQL message contents differ'
            milestones['explicit_sync_returned'] = await sync
            await asyncio.gather(a.command('space', 'sync', timeout=600), b.command('space', 'sync', timeout=600))
            milestones['drained'] = time.monotonic() - started
            verify_started = time.monotonic()
            if pref_count:
                preferences = await call(b, 'preferences.ttsConfigManager', 'getAllTtsConfigs')
                actual = {record['id']: record for record in preferences}
                assert all(actual.get(record['id']) == record for record in expected_preferences), 'Preference contents differ'
            local, remote = await asyncio.gather(call(a, 'application', 'syncConsistency'),
                                                 call(b, 'application', 'syncConsistency'))
            for report in (local, remote):
                assert not report['errors'] and not report['pendingOwners'] and not report['skipped'], report
            def contents(report):
                return {json.dumps(m['owner'], sort_keys=True): m['digest'] for m in report['manifests']}
            assert contents(local) == contents(remote), 'Materialized owner contents differ'
            row = dict(scenario=scenario, sql_shape=args.sql_shape, prepare_seconds=prepare_seconds,
                       milestones=milestones, missing_operations=inventory,
                       preference_observation='full validation after drain; no visibility polling',
                       message_payload_bytes=sql_count * args.message_bytes, file_count=file_count,
                       extra_verification_seconds=time.monotonic()-verify_started,
                       owners=len(local['manifests']), sql_api_writes=sql_count,
                       preference_api_creates=pref_count, file_bytes=file_count * args.file_bytes)
            results.append(row)
            (run.directory / 'backlog-results.json').write_text(json.dumps(results, indent=2))
            run.event('scenario-pass', **row)
        run.event('ALL-PASS')
    except Exception as error:
        run.event('FAIL', error=repr(error))
        await inspect_nodes(run)
        raise
    finally:
        for node in reversed(run.sessions):
            await node.stop(crash=True)
        print('ARTIFACT_DIRECTORY=' + str(run.directory), flush=True)


if __name__ == '__main__':
    asyncio.run(main())
