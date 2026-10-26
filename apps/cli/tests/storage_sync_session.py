"""Verify built-in storage roles with two real CLI processes and actual socket traffic.

Usage: python apps/cli/tests/storage_sync_session.py --binary PATH --transport tcp
Each run preserves isolated runtime directories, process logs and command responses.
"""
import argparse
import asyncio
import json
import sys
import time
import types
import uuid
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / 'apps/cli/tests'))
from space_stress_session import Run


async def inspect_nodes(run):
    """Capture each live process's policy, routing and synchronization diagnosis."""
    for session in run.sessions:
        if session.process is not None and session.process.returncode is None:
            for command in [('space', 'show'), ('control', 'show'), ('peers',), ('space', 'sync')]:
                try:
                    value = await session.command(*command, error=None, timeout=12)
                    run.event('diagnosis', node=session.name, command=command, result=value)
                except Exception as error:
                    run.event('diagnosis-error', node=session.name, command=command, error=str(error))


def identity_root(session, storage_root):
    """Resolve one identity-scoped test directory from the CLI's actual storage configuration."""
    config = json.loads((session.config / 'storage.json').read_text(encoding='utf-8'))
    return Path(config[storage_root]) / 'identities' / config['activeIdentityId']


async def wait_file(path, expected, timeout):
    """Wait for exact replicated bytes or an explicitly expected deletion."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if expected is None:
            if not path.exists():
                return
        elif path.is_file() and path.read_bytes() == expected:
            return
        await asyncio.sleep(0.1)
    raise AssertionError(f'Workspace replication timed out: {path}')


async def exercise(binary, transport, timeout):
    """Check default admission, user and relay replication, and restart recovery."""
    args = types.SimpleNamespace(binary=binary, transport=transport,
                                 focus='all', settle_seconds=timeout, operations=1)
    run = Run(args)
    try:
        a = await run.node('A')
        b = await run.node('B')
        a_seed = await a.command('core', 'chat', 'new')
        b_seed = await b.command('core', 'chat', 'new')
        await run.stage('real-pairing', lambda: run.pair(b, a))
        await run.stage('real-space-admission', lambda: run.join(b, a, a, [a, b]))
        await asyncio.gather(
            b.wait(('core', 'chat', 'list'), lambda rows: any(row['id'] == a_seed['chatId'] for row in rows)),
            a.wait(('core', 'chat', 'list'), lambda rows: any(row['id'] == b_seed['chatId'] for row in rows)),
        )
        run.event('PASS', scenario='default-role-bidirectional-existing-chat-sync')
        marker = uuid.uuid4().hex
        await a.command('space', 'preference-set', b.node, 'live-diagnostic', json.dumps(marker))
        await b.wait(('space', 'preferences', b.node), lambda value: value.get('live-diagnostic') == marker)
        run.event('PASS', scenario='automatic-device-preferences-default-user')
        chat = await a.command('core', 'chat', 'new')
        state = await b.command('control', 'show')
        assert state['deviceIdentityIds'][b.node] == 'user', state
        await b.wait(('core', 'chat', 'list'), lambda rows: any(row['id']==chat['chatId'] for row in rows))
        run.event('PASS', scenario='default-user-automatic-chat-sync', chatId=chat['chatId'])
        for source, target, image, media in [(a, b, 37, 41), (b, a, 43, 47)]:
            await source.command('core', 'prefs', 'media-history', str(image), str(media))
            await target.wait(('core', 'prefs', 'show'), lambda value: (value['maxImageHistoryUserTurns'], value['maxMediaHistoryUserTurns'])==(image,media))
            chat = await source.command('core', 'chat', 'new')
            await target.wait(('core', 'chat', 'list'), lambda rows: any(row['id']==chat['chatId'] for row in rows))
            run.event('PASS', scenario=f'automatic-business-sync-{source.name}-to-{target.name}', chatId=chat['chatId'])
        local_file = identity_root(a, 'workspaceRoot') / 'storage-sync' / '中文.bin'
        remote_file = identity_root(b, 'workspaceRoot') / 'storage-sync' / '中文.bin'
        local_file.parent.mkdir(parents=True, exist_ok=True)
        payload = bytes(range(256)) * 600
        local_file.write_bytes(payload)
        await wait_file(remote_file, payload, timeout)
        remote_file.write_bytes(b'edited on B')
        await wait_file(local_file, b'edited on B', timeout)
        renamed_local = local_file.with_name('renamed.bin')
        renamed_remote = remote_file.with_name('renamed.bin')
        local_file.rename(renamed_local)
        await wait_file(renamed_remote, b'edited on B', timeout)
        await wait_file(remote_file, None, timeout)
        renamed_remote.unlink()
        await wait_file(renamed_local, None, timeout)
        run.event('PASS', scenario='default-user-workspace-binary-edit-rename-delete')
        await a.command('control', 'identity', 'set', b.node, 'relay')
        await b.wait(('control', 'show'), lambda state: state['deviceIdentityIds'].get(b.node)=='relay')
        for source, target in [(a, b), (b, a)]:
            chat = await source.command('core', 'chat', 'new')
            await target.wait(('core', 'chat', 'list'), lambda rows: any(row['id']==chat['chatId'] for row in rows))
            run.event('PASS', scenario=f'relay-business-sync-{source.name}-to-{target.name}', chatId=chat['chatId'])
        await b.stop(crash=True)
        offline_file = identity_root(a, 'workspaceRoot') / 'storage-sync' / 'offline.bin'
        offline_file.write_bytes(b'written while B was stopped')
        chat = await a.command('core', 'chat', 'new')
        await b.start()
        await b.wait(('core', 'chat', 'list'), lambda rows: any(row['id']==chat['chatId'] for row in rows))
        await wait_file(identity_root(b, 'workspaceRoot') / 'storage-sync' / 'offline.bin', b'written while B was stopped', timeout)
        run.event('PASS', scenario='offline-chat-and-workspace-catchup-after-restart')
        run.event('ALL-PASS', transport=transport)
        return True
    except Exception as error:
        run.event('FAIL', transport=transport, error=str(error))
        await inspect_nodes(run)
        return False
    finally:
        for session in reversed(run.sessions):
            await session.stop(crash=True)
        run.events.close()


async def main():
    """Run the requested real socket transport and return a failing process status on failure."""
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--settle-seconds', type=float, default=25)
    parser.add_argument('--transport', choices=['tcp', 'http', 'ws'], default='tcp')
    args = parser.parse_args()
    return 0 if await exercise(args.binary, args.transport, args.settle_seconds) else 1


if __name__ == '__main__':
    raise SystemExit(asyncio.run(main()))
