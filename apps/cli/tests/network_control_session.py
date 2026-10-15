#!/usr/bin/env python3
"""Exercise two real CLI terminals, real sockets, and isolated native storage.

Usage: python3 apps/cli/tests/network_control_session.py [--transport tcp|http|ws]
No Flutter, mock router, direct database mutation, or per-command restart.
Lifecycle tests explicitly restart the same identities without pairing again.
"""
import argparse
import json
import os
from pathlib import Path
import pty
import select
import socket
import subprocess
import sys
import tempfile
import termios
import time


class Terminal:
    def __init__(self, binary, root, name, transport, home):
        self.name = name
        self.transport = transport
        self.root = root / name
        config = self.root / 'config'
        config.mkdir(parents=True)
        (config / 'storage.json').write_text(json.dumps({
            'runtimeRoot': str(self.root / 'runtime'),
            'workspaceRoot': str(self.root / 'workspaces'),
            'activeIdentityId': name,
            'identities': [{'id': name, 'name': name, 'createdAt': int(time.time() * 1000)}],
        }))
        with socket.socket() as reservation:
            reservation.bind(('127.0.0.1', 0))
            port = reservation.getsockname()[1]
        env = dict(os.environ, HOME=str(home), OPERIT_CLI_CONFIG_DIR=str(config))
        self.argv = [
            str(binary), 'cli', '--json', 'link', 'session', transport,
            '--bind', f'127.0.0.1:{port}', '--no-discovery',
        ]
        self.env = env
        self.log_path = root / f'{name}.log'
        self.start()

    def start(self):
        self.fd, slave = pty.openpty()
        attributes = termios.tcgetattr(slave)
        attributes[3] &= ~termios.ECHO
        termios.tcsetattr(slave, termios.TCSANOW, attributes)
        self.process = subprocess.Popen(self.argv, env=self.env,
                                        stdin=slave, stdout=slave, stderr=slave)
        os.close(slave)
        self.buffer = b''
        self.log = self.log_path.open('a')
        try:
            self.ready = self.read_json()
        except Exception:
            self.process.terminate()
            self.process.wait(timeout=10)
            os.close(self.fd)
            self.fd = None
            self.log.close()
            raise
        self.node = self.ready['nodeId']
        self.address = self.ready['bindAddress']
        if self.transport != 'tcp':
            self.address = f'{self.transport}://{self.address}/link'
        print(f'{self.name}: pid={self.process.pid} node={self.node} address={self.address}', flush=True)

    def read_json(self, timeout=90):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            while b'\n' in self.buffer:
                line, self.buffer = self.buffer.split(b'\n', 1)
                text = line.decode(errors='replace').strip()
                self.log.write(text + '\n')
                self.log.flush()
                try:
                    return json.loads(text)
                except json.JSONDecodeError:
                    pass
            if select.select([self.fd], [], [], 0.1)[0]:
                try:
                    data = os.read(self.fd, 65536)
                except OSError:
                    data = b''
                if not data:
                    raise AssertionError(f'CLI exited: {self.process.poll()}; see {self.log.name}')
                self.buffer += data
        raise AssertionError(f'CLI timeout; see {self.log.name}')

    def command(self, *args, error=False):
        self.log.write('> ' + json.dumps(args) + '\n')
        os.write(self.fd, (json.dumps(args) + '\n').encode())
        value = self.read_json()
        has_error = isinstance(value, dict) and 'error' in value
        assert has_error == error, (args, value)
        return value

    def wait(self, predicate, *args):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            value = self.command(*args)
            if predicate(value):
                return value
            time.sleep(0.1)
        raise AssertionError(f'Automatic sync did not converge: {args}')

    def close(self):
        if self.fd is None:
            return
        try:
            self.command('quit')
            self.process.wait(timeout=10)
            assert self.process.returncode == 0
        finally:
            if self.process.poll() is None:
                self.process.terminate()
                try:
                    self.process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait()
            os.close(self.fd)
            self.fd = None
            self.log.close()


def pair(source, target, transport):
    pending = source.command('pair-start', target.node, target.address, transport)
    prompt = next(p for p in target.command('prompts') if p['pairingId'] == pending['pairingId'])
    assert len(prompt['confirmationCode']) == 6 and prompt['confirmationCode'].isdigit()
    source.command('pair-finish', pending['pairingId'], prompt['confirmationCode'])


def join(a, b):
    before_a, before_b = a.command('space', 'show'), b.command('space', 'show')
    request = a.command('space', 'join', b.node)
    assert request['status'] == 'pending' and request['reviewerDeviceId'] == b.node
    incoming = b.command('space', 'requests', 'incoming')
    assert len(incoming) == 1 and incoming[0]['canApprove']
    assert incoming[0]['requestId'] == request['requestId']
    assert a.command('space', 'show') == before_a
    assert b.command('space', 'show') == before_b
    assert b.node not in a.command('control', 'show')['memberNodeIds']
    b.command('space', 'approve', request['requestId'], str(request['assignmentVersion']))
    assert a.command('space', 'refresh', request['requestId'])['status'] == 'joined'
    assert a.command('space', 'show') == b.command('space', 'show')
    assert a.command('control', 'show')['deviceIdentityIds'][a.node] == 'user'
    assert a.command('space', 'requests', 'incoming') == []
    return request


def exercise(a, b, transport):
    pair(a, b, transport)
    # One pairing only. Space admission, not a second pairing, enables return traffic.
    assert next(p for p in b.command('peers') if p['nodeId'] == a.node)['outbound'] is False
    # Allow the listener's handshake completion notification to reach its worker.
    time.sleep(0.2)
    # Core's Space status also reports membership: pairing is not joining.
    assert a.command('space', 'status', b.node)['status'] == 'RemovedFromSpace'
    assert b.command('space', 'status', a.node)['status'] == 'RemovedFromSpace'
    join(a, b)
    assert a.command('space', 'status', b.node)['status'] == 'Online'
    assert b.command('space', 'status', a.node)['status'] == 'Online'
    old_space = a.command('space', 'show')['spaceId']
    for terminal in (a, b):
        space = terminal.command('space', 'leave')
        control = terminal.command('control', 'show')
        assert space['spaceId'] != old_space and space['members'] == [terminal.node]
        assert control['initialized'] and control['deviceIdentityIds'][terminal.node] == 'admin'
        assert sum(row['summary'].startswith('Bootstrap control') for row in terminal.command('control', 'audit')) == 1
    # No process has restarted since startup. This is the leave lifecycle regression.
    refused = a.command('space', 'join', b.node)
    b.command('space', 'requests', 'incoming')
    b.command('space', 'approve', refused['requestId'], '0', error=True)
    b.command('space', 'reject', refused['requestId'], str(refused['assignmentVersion']))
    assert a.command('space', 'refresh', refused['requestId'])['status'] == 'rejected'
    cancelled = a.command('space', 'join', b.node)
    assert a.command('space', 'cancel', cancelled['requestId'])['status'] == 'cancelled'
    assert b.command('space', 'requests', 'incoming') == []
    join(a, b)
    a.command('control', 'policy', 'set', 'test-policy', 'forbidden', error=True)
    b.command('control', 'policy', 'set', 'test-policy', 'allowed')
    a.wait(lambda v: v.get('test-policy') == 'allowed', 'control', 'policy', 'list')
    # Actual business mutations; only polling reads here, no manual synchronizeOnce.
    chats = []
    for source, target, label, image, media in [(a, b, 'A', 37, 41), (b, a, 'B', 43, 47)]:
        source.command('core', 'prefs', 'media-history', str(image), str(media))
        target.wait(lambda v: (v['maxImageHistoryUserTurns'], v['maxMediaHistoryUserTurns']) == (image, media), 'core', 'prefs', 'show')
        chat = source.command('core', 'chat', 'new', '--group', f'CLI-{label}-{transport}-sync')
        chats.append((source, target, chat['chatId']))
        target.wait(lambda rows: any(row['id'] == chat['chatId'] for row in rows), 'core', 'chat', 'list')
        snapshots = target.command('chat-watch', chat['chatId'])
        assert snapshots['chatStateFlow']['currentChatId'] == chat['chatId']
        assert snapshots['chatStateFlow']['isLoading'] is False
        assert snapshots['chatMessagesFlow'] == []
        assert snapshots['route']['ownerNodeId'] == source.node
        assert snapshots['route']['activeNodeId'] == source.node
        assert snapshots['route']['activeIsLocal'] is False
        assert snapshots['route']['ownerReachable'] is True
    # Restore the same identities and listener ports. A starts first, before B
    # exists, so the first connect must fail and subsequent retries must work.
    identities = (a.node, b.node)
    space = a.command('space', 'show')
    a.close()
    b.close()
    a.start()
    assert a.command('space', 'status', identities[1])['status'] == 'Offline'
    a.command('core', 'prefs', 'media-history', '53', '59')
    time.sleep(6)
    b.start()
    assert (a.node, b.node) == identities
    for source, target in [(a, b), (b, a)]:
        source.wait(lambda v: v['status'] == 'Online', 'space', 'status', target.node)
        assert source.command('space', 'show') == space
    b.wait(lambda v: (v['maxImageHistoryUserTurns'], v['maxMediaHistoryUserTurns']) == (53, 59),
           'core', 'prefs', 'show')
    assert next(p for p in b.command('peers') if p['nodeId'] == a.node)['outbound'] is False
    for source, target, chat_id in chats:
        snapshots = target.command('chat-watch', chat_id)
        assert snapshots['chatStateFlow']['currentChatId'] == chat_id
        assert snapshots['route']['activeNodeId'] == source.node
        assert snapshots['route']['activeIsLocal'] is False
        assert snapshots['route']['ownerReachable'] is True

    # Peer exit must remove online evidence, then restarting either side must
    # reconnect automatically and deliver changes made while it was offline.
    for offline, survivor, image, media in [(b, a, 61, 67), (a, b, 71, 73)]:
        offline.close()
        survivor.wait(lambda v: v['status'] == 'Offline', 'space', 'status', offline.node)
        survivor.command('core', 'prefs', 'media-history', str(image), str(media))
        offline.start()
        for source, target in [(offline, survivor), (survivor, offline)]:
            source.wait(lambda v: v['status'] == 'Online', 'space', 'status', target.node)
        offline.wait(lambda v: (v['maxImageHistoryUserTurns'], v['maxMediaHistoryUserTurns']) == (image, media),
                     'core', 'prefs', 'show')
    print(f'PASS {transport}: persisted identities, staggered restart, offline detection, either-side reconnect and queued automatic sync', flush=True)
    role = b.command('control', 'identity', 'define', 'Reviewer', 'approve', 'join', 'view')
    b.command('control', 'identity', 'set', a.node, 'Reviewer')
    a.wait(lambda v: v['deviceIdentityIds'].get(a.node) == role['roleId'], 'control', 'show')
    b.command('control', 'identity', 'clear', a.node)
    b.command('control', 'identity', 'set', a.node, 'User')
    a.wait(lambda v: v['deviceIdentityIds'].get(a.node) == 'user', 'control', 'show')
    assert a.command('space', 'sync')['synchronized']
    assert b.command('space', 'sync')['synchronized']
    b.command('control', 'device', 'disconnect', a.node)
    assert a.node in b.command('control', 'show')['disconnectedNodeIds']
    b.command('control', 'device', 'remove', a.node)
    assert a.node not in b.command('control', 'show')['memberNodeIds']
    print(f'PASS {transport}: pairing, approvals, leave/rejoin, reject/cancel/stale decision, roles/policy, bidirectional automatic prefs/chat sync and remote chat watches, disconnect/remove', flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--transport', choices=['tcp', 'http', 'ws'], default='tcp')
    parser.add_argument('--binary', type=Path, default=Path(__file__).resolve().parents[1] / 'target/release/operit2')
    args = parser.parse_args()
    root = Path(tempfile.mkdtemp(prefix=f'operit-cli-{args.transport}-'))
    home = root / 'home'
    home.mkdir()
    print(f'Test artifacts: {root}', flush=True)
    if sys.platform == 'darwin':
        # Test-only login Keychain selected through the isolated HOME; never
        # unlock, authorize, read, or change the user's real login Keychain.
        env = dict(os.environ, HOME=str(home))
        keychain = home / 'Library/Keychains/login.keychain-db'
        keychain.parent.mkdir(parents=True)
        original = subprocess.check_output(['security', 'default-keychain', '-d', 'user'])
        subprocess.run(['security', 'create-keychain', '-p', str(uuid.uuid4()), str(keychain)], env=env, check=True)
        assert subprocess.check_output(['security', 'default-keychain', '-d', 'user']) == original
    terminals = []
    try:
        for name in ('A', 'B'):
            terminals.append(Terminal(args.binary.resolve(), root, name, args.transport, home))
        exercise(*terminals, args.transport)
    finally:
        for terminal in terminals:
            terminal.close()
    if sys.platform == 'darwin':
        assert subprocess.check_output(['security', 'default-keychain', '-d', 'user']) == original


if __name__ == '__main__':
    main()
