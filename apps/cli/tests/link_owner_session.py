#!/usr/bin/env python3
"""Real listener plus separate management processes: approval, rejoin and ownership.

Run with the project venv and --binary pointing to the current CLI executable.
Profiles and file secrets are isolated by the existing transport test harness.
"""
import argparse
import asyncio
import json
import os
from pathlib import Path

from space_stress_session import REPO, Run, Session


class ListenerSession(Session):
    """Manage a listening Core only through independent CLI processes."""

    def environment(self):
        """Select this test identity without inheriting an SDK endpoint override."""
        return {**{key: value for key, value in os.environ.items()
                   if key not in ('OPERIT_PLUGIN_SDK_SOCKET_DIR', 'OPERIT_PLUGIN_SDK_INSTANCE')},
                'OPERIT_CLI_CONFIG_DIR': str(self.config), 'OPERIT_CLI_SECRET_STORE': 'file'}

    async def invoke(self, *args, config=None):
        """Collect one bounded management process and its exact exit status."""
        environment = self.environment()
        if config is not None:
            environment['OPERIT_CLI_CONFIG_DIR'] = str(config)
        process = await asyncio.create_subprocess_exec(
            str(self.run.binary), *args, cwd=REPO, env=environment,
            stdout=asyncio.subprocess.PIPE, stderr=self.stderr)
        try:
            output, _ = await asyncio.wait_for(process.communicate(), 60)
        except BaseException:
            process.kill()
            await process.wait()
            raise
        self.log.write(json.dumps({'external': args, 'exit': process.returncode,
                                   'stdout': output.decode(errors='replace')}) + '\n')
        self.log.flush()
        return process.returncode, output.decode()

    async def start(self, bind=None):
        """Start the production listen path, including its ownership/control endpoint."""
        self.process = await asyncio.create_subprocess_exec(
            str(self.run.binary), 'cli', '--json', 'link', 'listen', self.transport,
            '--bind', '127.0.0.1:0', '--automatic-port', '--no-discovery',
            cwd=REPO, env=self.environment(), stdin=asyncio.subprocess.DEVNULL,
            stdout=asyncio.subprocess.PIPE, stderr=self.stderr)
        ready = await self.read(90)
        assert ready.get('listening'), ready
        self.node, self.bind = ready['nodeId'], ready['bindAddress']

    async def command(self, *args, error=False, timeout=45):
        """Expose the harness contract using public Link management commands."""
        if args[0] == 'call':
            assert args[1:3] == ('server.runtimeRemoteLinkService', 'decideDeviceSpaceJoin'), args
            payload = json.loads(args[3])
            assert payload['approve'] is True
            args = ('space', 'approve')
        status, output = await self.invoke('cli', '--json', 'link', *args)
        assert status == 0, (args, status, output)
        value = json.loads(output)
        assert not (isinstance(value, dict) and 'error' in value), value
        return value

    async def stop(self, crash=False):
        """Release the owner normally so the same identity can start again."""
        if self.process is None or self.process.returncode is not None:
            return
        status, output = await self.invoke('daemon', 'stop', '--json')
        assert status == 0, output
        await asyncio.wait_for(self.process.wait(), 20)
        assert self.process.returncode == 0, (self.process.returncode,
                                            await self.process.stdout.read())


class OwnerRun(Run):
    """Repeat the reported lifecycle with a live cached host and independent clients."""

    async def exercise(self):
        host = ListenerSession(self, 'host', self.args.transport)
        self.sessions.append(host)
        await host.start()
        client = await self.node('client')
        initial = await host.command('space', 'show')
        await self.pair(client, host)

        for iteration in range(self.args.rounds):
            host_peers = await host.command('peers')
            client_peers = await client.command('peers')
            assert len(host_peers) == len(client_peers) == 1
            assert host_peers[0]['inbound'] is True and host_peers[0]['outbound'] is False
            assert client_peers[0]['outbound'] is True and client_peers[0]['inbound'] is False
            before = await client.command('space', 'show')
            pending = await client.command('space', 'join')
            await host.wait(('space', 'requests', 'incoming'), lambda rows: any(
                row['requestId'] == pending['requestId'] for row in rows))
            for _ in range(2):
                status, output = await host.invoke('cli', '--json', 'link', 'space', 'cancel',
                                                 config=client.config)
                cancelled = json.loads(output)
                assert status == 0 and cancelled['requestId'] == pending['requestId'], output
                assert cancelled['status'] == 'cancelled', cancelled
            assert await client.command('space', 'show') == before
            request = await self.join(client, host, host, [host, client])
            assert request['requestId'] != pending['requestId']
            assert (await host.command('space', 'show'))['spaceId'] == initial['spaceId']
            marker = f'owner-round-{iteration}'
            await host.command('space', 'preference-set', host.node, 'owner-regression', json.dumps(marker))
            await client.command('space', 'preference-set', client.node, 'owner-regression', json.dumps(marker))
            assert (await host.command('space', 'preferences', host.node)).get('owner-regression') == marker
            assert (await client.command('space', 'preferences', client.node)).get('owner-regression') == marker
            await asyncio.gather(
                client.wait(('space', 'preferences', host.node), lambda v: v.get('owner-regression') == marker),
                host.wait(('space', 'preferences', client.node), lambda v: v.get('owner-regression') == marker))
            # These commands used to create independent cached Cores next to listen.
            outputs = await asyncio.gather(*(
                host.command('space', 'requests', 'incoming') for _ in range(4)))
            assert all(not rows for rows in outputs), outputs
            text_status, text = await host.invoke('cli', 'link', 'space', 'show')
            assert text_status == 0 and text.startswith('Device space: '), text
            assert (await host.command('space', 'show'))['spaceId'] == initial['spaceId']
            status, output = await host.invoke('daemon', 'status', '--json')
            state = json.loads(output)['status']
            assert status == 0 and state['pid'] == host.process.pid and state['mode'] == 'listen', state
            assert state['commands'] >= 6, state
            status, _ = await host.invoke('cli', '--json', 'link', 'listen', 'tcp', '--duration-ms', '1')
            assert status != 0, 'a second listener must not open the same identity'
            self.event('owner-join', iteration=iteration, request=request['requestId'], pid=host.process.pid)
            if iteration + 1 < self.args.rounds:
                for session in (client, host):
                    if await session.command('peers'):
                        await session.command('unpair')
                await client.command('space', 'leave')
                await self.pair(client, host)

        await host.stop()
        await host.start()
        assert (await host.command('space', 'show'))['spaceId'] == initial['spaceId']
        self.event('owner-restart', space=initial['spaceId'])


def main():
    """Run the regression against an explicitly selected current binary."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--transport', choices=['tcp', 'http', 'ws'], default='tcp')
    parser.add_argument('--rounds', type=int, default=3)
    args = parser.parse_args()
    args.focus, args.topology = 'owner', 'pair'
    args.settle_seconds, args.operations, args.stall_seconds, args.replicas = 45, 0, 0, False
    assert args.rounds >= 2
    raise SystemExit(asyncio.run(OwnerRun(args).run()))


if __name__ == '__main__':
    main()
