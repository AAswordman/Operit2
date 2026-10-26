#!/usr/bin/env python3
"""Real CLI processes/sockets: group merge, five hops, churn and listener migration.

Uses isolated profiles and file secrets. No mocked routers, DB edits, discovery or
changes to the host network. SIGSTOP models stalled traffic; SIGKILL models a
failed relay. IPv4/IPv6 listener migration is not a physical LAN handover.
The directed focuses use macOS process sandboxes to prohibit reverse TCP dials.
"""
import argparse
import asyncio
import json
import os
from pathlib import Path
import signal
import shutil
import socket
import statistics
import sys
import tempfile
import time
import traceback
import uuid

REPO = Path(__file__).resolve().parents[3]


class Session:
    def __init__(self, run, name, transport):
        self.run, self.name, self.transport = run, name, transport
        self.directory = run.directory / name
        self.config = self.directory / 'config'
        self.config.mkdir(parents=True)
        identity = uuid.uuid4().hex
        (self.config / 'storage.json').write_text(json.dumps({
            'runtimeRoot': str(self.directory / 'runtime'),
            'workspaceRoot': str(self.directory / 'workspaces'),
            'activeIdentityId': identity,
            'identities': [{'id': identity, 'name': f'Space stress {name}', 'createdAt': int(time.time() * 1000)}],
        }))
        self.bind, self.node, self.process = '127.0.0.1:0', None, None
        self.lock, self.paused = asyncio.Lock(), False
        self.log = (self.directory / 'session.jsonl').open('a', encoding='utf-8')
        self.stderr = (self.directory / 'stderr.log').open('ab')

    async def start(self, bind=None):
        assert self.process is None or self.process.returncode is not None
        address = bind or self.bind
        if address.endswith(':0'):
            host = address.rsplit(':', 1)[0].strip('[]')
            with socket.socket(socket.AF_INET6 if ':' in host else socket.AF_INET) as reservation:
                reservation.bind((host, 0))
                address = f'[{host}]:{reservation.getsockname()[1]}' if ':' in host else f'{host}:{reservation.getsockname()[1]}'
        self.process = await asyncio.create_subprocess_exec(
            *self.run.command_prefix(self),
            str(self.run.binary), 'cli', '--json', 'link', 'session', self.transport,
            '--bind', address, '--fixed-port', '--no-discovery', cwd=REPO,
            env={**{key:value for key,value in os.environ.items() if key not in ('OPERIT_PLUGIN_SDK_SOCKET_DIR', 'OPERIT_PLUGIN_SDK_INSTANCE')},
                 'OPERIT_CLI_CONFIG_DIR': str(self.config), 'OPERIT_CLI_SECRET_STORE': 'file'},
            stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE, stderr=self.stderr,
            limit=4 * 1024 * 1024,
        )
        ready = await self.read(90)
        assert ready.get('listening'), ready
        assert self.node is None or ready['nodeId'] == self.node, 'restart changed device identity'
        self.node, self.bind = ready['nodeId'], ready['bindAddress']
        self.run.event('start', node=self.name, pid=self.process.pid, identity=self.node, bind=self.bind, transport=self.transport)

    @property
    def address(self):
        return self.bind if self.transport == 'tcp' else f'{self.transport}://{self.bind}/link'

    async def read(self, timeout):
        async def next_json():
            while True:
                line = await self.process.stdout.readline()
                if not line:
                    raise AssertionError(f'{self.name} exited: {self.process.returncode}')
                self.log.write(line.decode(errors='replace')); self.log.flush()
                try:
                    return json.loads(line)
                except json.JSONDecodeError:
                    pass
        return await asyncio.wait_for(next_json(), timeout)

    async def command(self, *args, error=False, timeout=45):
        async with self.lock:
            assert self.process.returncode is None and not self.paused, self.name
            self.log.write(json.dumps({'command': args, 'at': time.time()}) + '\n'); self.log.flush()
            start = time.monotonic()
            self.process.stdin.write((json.dumps(args) + '\n').encode())
            await self.process.stdin.drain()
            value = await self.read(timeout)
            elapsed = time.monotonic() - start
            self.run.latencies.append(elapsed)
            self.run.command_counts[args[0]] = self.run.command_counts.get(args[0], 0) + 1
            assert error is None or (isinstance(value, dict) and 'error' in value) == error, (self.name, args, value)
            return value

    async def wait(self, args, predicate, timeout=None, allow_error=False):
        deadline, last = time.monotonic() + (timeout or self.run.args.settle_seconds), None
        while time.monotonic() < deadline:
            last = await self.command(*args, error=None if allow_error else False)
            if predicate(last):
                return last
            await asyncio.sleep(.25)
        raise AssertionError(f'{self.name} did not converge: {args}: {last}')

    def pause(self):
        self.process.send_signal(signal.SIGSTOP); self.paused = True

    def resume(self):
        if self.paused:
            self.process.send_signal(signal.SIGCONT); self.paused = False

    async def stop(self, crash=False):
        if self.process is None or self.process.returncode is not None:
            return
        self.resume()
        if crash:
            self.process.kill()
        else:
            self.process.stdin.write(b'["quit"]\n')
            await self.process.stdin.drain()
        try:
            await asyncio.wait_for(self.process.wait(), 15)
        except asyncio.TimeoutError:
            self.process.kill(); await self.process.wait()
            if not crash:
                raise AssertionError(f'{self.name} graceful shutdown hung')
        if not crash:
            assert self.process.returncode == 0, (self.name, self.process.returncode)


class Run:
    def __init__(self, args):
        self.binary, self.args = args.binary.resolve(), args
        # Keep artifact paths short; CLI profiles own their IPC isolation.
        self.directory = Path(tempfile.mkdtemp(prefix='operit-space-stress-', dir='/tmp' if sys.platform=='darwin' else None))
        shutil.copyfile(__file__, self.directory / 'runner.py')
        self.events = (self.directory / 'events.jsonl').open('a')
        self.latencies, self.sessions, self.pairings, self.results = [], [], set(), []
        self.command_counts, self.dial_targets = {}, {}
        self.started = time.monotonic()
        self.event('artifacts', path=str(self.directory))

    def event(self, stage, **fields):
        value = {'stage': stage, 'elapsed_s': round(time.monotonic() - self.started, 3), **fields}
        self.events.write(json.dumps(value) + '\n'); self.events.flush()
        print(json.dumps(value), flush=True)

    async def node(self, name):
        transport = self.args.transport if self.args.transport != 'mixed' else ['tcp', 'http', 'ws'][len(self.sessions) % 3]
        session = Session(self, name, transport); self.sessions.append(session)
        await session.start()
        return session

    def directed_targets(self, session):
        if session.name in self.dial_targets:
            return [s for s in self.sessions if s.name in self.dial_targets[session.name]]
        predecessor = dict(zip('BCDEF', 'ABCDE')).get(session.name, 'F')
        if session.name == 'A' and self.args.focus != 'directed-ring':
            return []
        return [next(s for s in self.sessions if s.name == predecessor)]

    def command_prefix(self, session):
        if self.args.focus not in ('directed', 'directed-ring', 'departure', 'silent-removal', 'directed-pair'):
            return []
        # Restrict the actual CLI process, including all of its native network hosts.
        # Accepted streams can reply, but may not create a reverse TCP connection.
        targets = self.directed_targets(session)
        policy = ['(version 1)', '(allow default)', '(deny network-outbound (remote tcp))',
                  '(deny network-outbound (remote udp))']
        policy += [f'(allow network-outbound (remote tcp "localhost:{target.bind.rsplit(":", 1)[1]}"))' for target in targets]
        path = session.directory / 'network.sb'
        path.write_text('\n'.join(policy) + '\n')
        self.event('sandbox', node=session.name, allowed=[target.name for target in targets], profile=str(path))
        return ['/usr/bin/sandbox-exec', '-f', str(path)]

    async def verify_directed_policy(self, session):
        allowed = [s.bind for s in self.directed_targets(session)]
        probe = """import errno,json,socket,sys
result=[]
for address in sys.argv[1:]:
 host,port=address.rsplit(':',1)
 try:
  with socket.create_connection((host,int(port)),timeout=3): pass
  result.append('connected')
 except OSError as error: result.append(error.errno)
print(json.dumps(result))
"""
        process = await asyncio.create_subprocess_exec(
            '/usr/bin/sandbox-exec', '-f', str(session.directory / 'network.sb'),
            sys.executable, '-c', probe, *allowed, session.bind,
            stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
        out, err = await asyncio.wait_for(process.communicate(), 10)
        result = json.loads(out)
        assert process.returncode == 0 and result == ['connected'] * len(allowed) + [1], (session.name, result, err.decode())
        self.event('policy-verified', node=session.name, allowed=allowed, denied=session.bind, result=result)

    async def pair(self, source, target):
        pending = await source.command('pair-start', target.node, target.address, target.transport)
        prompts = await target.command('prompts')
        prompt = next(p for p in prompts if p['pairingId'] == pending['pairingId'])
        await source.command('pair-finish', pending['pairingId'], prompt['confirmationCode'])
        self.pairings.add(frozenset((source.node, target.node)))

    async def join(self, source, gateway, reviewer, members):
        before = await source.command('space', 'show')
        request = await source.command('space', 'join', gateway.node)
        assert request['status'] == 'pending', request
        if request['reviewerDeviceId'] != reviewer.node:
            request = await source.wait(('space', 'refresh', request['requestId']),
                lambda value: value.get('reviewerDeviceId') == reviewer.node)
        assert request['status'] == 'pending', request
        assert await source.command('space', 'show') == before, 'pairing/request merged membership before approval'
        incoming = await reviewer.wait(('space', 'requests', 'incoming'),
            lambda rows: any(r['requestId'] == request['requestId'] and r['canApprove'] for r in rows))
        row = next(r for r in incoming if r['requestId'] == request['requestId'])
        await reviewer.command('space', 'approve', row['requestId'], str(row['assignmentVersion']))
        await source.wait(('space', 'refresh', request['requestId']), lambda v: v['status'] == 'joined')
        await self.members(members)
        return request

    async def members(self, sessions):
        expected = {s.node for s in sessions}
        async def check(s):
            space = await s.wait(('space', 'show'), lambda v: set(v['members']) == expected)
            state = await s.wait(('control', 'show'), lambda v: set(v['memberNodeIds']) == expected)
            assert set(state['deviceIdentityIds']) == expected, (s.name, state)
            return space['spaceId']
        spaces = await asyncio.gather(*(check(s) for s in sessions))
        assert len(set(spaces)) == 1, spaces

    async def routes(self, source, target, online, hops=None):
        rows = await source.wait(('control', 'device', 'list'), lambda rows: any(
            r['deviceId'] == target.node and r['online'] == online and
            (hops is None or (hops == 1 and r.get('relayHops') is None) or r.get('relayHops') == hops) for r in rows))
        row = next(r for r in rows if r['deviceId'] == target.node)
        if hops and hops > 1:
            assert row['relayPath'][-1] == target.node and len(row['relayPath']) == hops, row
        return row

    async def no_shortcuts(self, nodes):
        for s in nodes:
            for peer in await s.command('peers'):
                assert frozenset((s.node, peer['nodeId'])) in self.pairings, (s.name, peer)

    async def pressure(self, nodes, iteration):
        async def worker(s):
            for n in range(self.args.operations):
                await s.command('space', 'preference-set', s.node, 'stress-local', json.dumps({'round': iteration, 'node': s.name, 'n': n}))
                await s.command('control', 'device', 'list')
                await s.command('space', 'show')
        await asyncio.gather(*(worker(s) for s in nodes))
        await self.no_shortcuts(nodes)
        self.event('pressure', iteration=iteration, commands=3 * len(nodes) * self.args.operations)

    async def stage(self, name, action, required=True):
        self.event('begin', scenario=name)
        start = time.monotonic()
        try:
            await action()
        except Exception as e:
            self.results.append({'scenario': name, 'passed': False, 'error': str(e)})
            self.event('FAIL', scenario=name, error=str(e))
            if required:
                raise
            return False
        else:
            self.results.append({'scenario': name, 'passed': True, 'seconds': round(time.monotonic()-start, 3)})
            self.event('PASS', scenario=name)
            return True

    async def exercise_directed(self):
        if self.args.focus == 'directed-ring':
            # The first process must know the last process's port before starting.
            # Reserve all six ports together so none can be reused for a neighbor.
            reservations = []
            try:
                for index, name in enumerate('ABCDEF'):
                    transport = ['tcp', 'http', 'ws'][index % 3] if self.args.transport == 'mixed' else self.args.transport
                    session = Session(self, name, transport)
                    self.sessions.append(session)
                    reservation = socket.socket()
                    reservations.append(reservation)
                    reservation.bind(('127.0.0.1', 0))
                    session.bind = f'127.0.0.1:{reservation.getsockname()[1]}'
                nodes = list(self.sessions)
                for session, reservation in zip(nodes, reservations):
                    reservation.close()
                    await session.start()
            finally:
                for reservation in reservations:
                    reservation.close()
        else:
            nodes = [await self.node(name) for name in 'ABCDEF']
        a,b,c,d,e,f = nodes
        await self.stage('os-enforces-directed-neighbors', lambda: asyncio.gather(*(self.verify_directed_policy(s) for s in nodes)))

        async def admit_chain():
            for index in range(1, len(nodes)):
                await self.pair(nodes[index], nodes[index-1])
                await self.join(nodes[index], nodes[index-1], a, nodes[:index+1])
            await asyncio.gather(self.routes(a,f,True,5), self.routes(f,a,True,5))
            await self.no_shortcuts(nodes)
            if self.args.replicas:
                role = await a.command('control','identity','define','DirectedReplica','view','relay','storage','execute')
                for node in nodes[1:]:
                    await a.command('control','identity','set',node.node,role['roleId'])
                await asyncio.gather(*(node.wait(('control','show'),lambda v,node=node:v['deviceIdentityIds'].get(node.node)==role['roleId']) for node in nodes[1:]))
        await self.stage('directed-five-hop-admission', admit_chain)
        await self.stage('directed-concurrent-operations', lambda: self.pressure(nodes,0))

        async def sync():
            marker = uuid.uuid4().hex
            await a.command('space','preference-set',f.node,'directed-forward',json.dumps(marker))
            await f.command('space','preference-set',a.node,'directed-reverse',json.dumps(marker))
            await asyncio.gather(
                f.wait(('space','preferences',f.node),lambda v:v.get('directed-forward')==marker),
                a.wait(('space','preferences',a.node),lambda v:v.get('directed-reverse')==marker))
            if self.args.replicas:
                chat = await a.command('core','chat','new')
                await f.wait(('core','chat','list'),lambda rows:any(row['id']==chat['chatId'] for row in rows))
                await f.wait(('chat-watch',chat['chatId']),lambda v:v.get('route',{}).get('ownerReachable') is True,allow_error=True)
        await self.stage('directed-five-hop-preferences-chat-watch', sync)
        if self.args.focus == 'directed-ring':
            return await self.exercise_directed_ring(nodes)

        g = await self.node('G')
        await self.verify_directed_policy(g)
        await self.pair(g,f)
        before = await g.command('space','show')
        async def pending():
            request = await g.command('space','join',f.node)
            assert request['status']=='pending', request
            incoming = await a.wait(('space','requests','incoming'), lambda rows:any(row['requestId']==request['requestId'] and row['canApprove'] for row in rows))
            return next(row for row in incoming if row['requestId']==request['requestId'])
        async def assert_cancelled(row):
            await g.wait(('space','refresh',row['requestId']),lambda v:v.get('status')=='cancelled',allow_error=True)
            await a.command('space','approve',row['requestId'],str(row['assignmentVersion']),error=True)
            assert await g.command('space','show') == before
            await self.members(nodes)

        async def cancel_reapply():
            previous = None
            for _ in range(self.args.rounds):
                row = await pending()
                assert row['requestId'] != previous
                previous = row['requestId']
                result = await g.command('space','cancel',row['requestId'])
                assert result['status']=='cancelled', result
                # Repeat the cancellation and restart the actual applicant process.
                assert (await g.command('space','cancel',row['requestId']))['status']=='cancelled'
                await g.stop(crash=True); await g.start()
                await assert_cancelled(row)
        await self.stage('directed-six-hop-cancel-reapply-restart', cancel_reapply)

        async def cancel_gateway_down():
            row = await pending()
            await f.stop(crash=True)
            await g.command('space','cancel',row['requestId'],error=True)
            await g.stop(crash=True); await g.start()
            # A failed cancellation must survive restart and block a replacement submission.
            await g.command('space','join',f.node,error=True)
            await f.start()
            await assert_cancelled(row)
            await asyncio.gather(self.routes(a,f,True,5),self.routes(f,a,True,5))
        await self.stage('directed-cancel-offline-gateway-and-restart-applicant', cancel_gateway_down)

        async def cancel_partitioned_reviewer():
            row = await pending()
            await c.stop(crash=True)
            marker = uuid.uuid4().hex
            await a.command('space','preference-set',f.node,'directed-offline',json.dumps(marker))
            assert (await g.command('space','cancel',row['requestId']))['status']=='cancelled'
            await g.stop(crash=True); await g.start()
            await c.start()
            await assert_cancelled(row)
            await f.wait(('space','preferences',f.node),lambda v:v.get('directed-offline')==marker)
            await sync()
        await self.stage('directed-cancel-while-reviewer-partitioned-and-offline-sync', cancel_partitioned_reviewer)
        await g.stop()

        async def restart_chain():
            await asyncio.gather(*(node.stop(crash=True) for node in nodes))
            for node in reversed(nodes):
                await node.start()
            await asyncio.gather(self.routes(a,f,True,5), self.routes(f,a,True,5))
            await self.members(nodes)
            await sync()
        await self.stage('directed-all-killed-reverse-order-restart', restart_chain)

        expected = {node.node for node in nodes}
        async def race(index):
            applicant = await self.node(f'R{index}')
            await self.verify_directed_policy(applicant)
            await self.pair(applicant,f)
            local = await applicant.command('space','show')
            request = await applicant.command('space','join',f.node)
            rows = await a.wait(('space','requests','incoming'),lambda rows:any(row['requestId']==request['requestId'] and row['canApprove'] for row in rows))
            row = next(row for row in rows if row['requestId']==request['requestId'])
            delay = [0, .002, .01, .03][index % 4]
            async def cancel():
                await asyncio.sleep(delay)
                return await applicant.command('space','cancel',row['requestId'],error=None)
            cancelled, approved = await asyncio.gather(
                cancel(), a.command('space','approve',row['requestId'],str(row['assignmentVersion']),error=None))
            final = await applicant.wait(('space','refresh',row['requestId']),lambda v:v.get('status') in ('cancelled','joined'),allow_error=True)
            if final['status']=='joined':
                assert 'error' not in approved, approved
                expected.add(applicant.node)
                await applicant.wait(('space','show'),lambda v:set(v['members'])==expected)
            else:
                assert 'error' in approved, approved
                assert await applicant.command('space','show') == local
                await a.command('space','approve',row['requestId'],str(row['assignmentVersion']),error=True)
            await asyncio.gather(*(node.wait(('space','show'),lambda v:set(v['members'])==expected) for node in nodes))
            self.event('race-outcome', iteration=index, delay_s=delay, status=final['status'],
                cancellation=cancelled.get('status',cancelled.get('error','').split('\n')[0]),
                approval=approved.get('status',approved.get('error','').split('\n')[0]))
            await applicant.stop()
        for index in range(self.args.rounds):
            await self.stage(f'directed-six-hop-concurrent-cancel-approve-{index}', lambda index=index: race(index))

        async def concurrent_applicants():
            applicants = await asyncio.gather(*(self.node(f'P{index}') for index in range(max(4,self.args.rounds))))
            await asyncio.gather(*(self.verify_directed_policy(applicant) for applicant in applicants))
            await asyncio.gather(*(self.pair(applicant,f) for applicant in applicants))
            requests = await asyncio.gather(*(applicant.command('space','join',f.node) for applicant in applicants))
            ids = {request['requestId'] for request in requests}
            rows = await a.wait(('space','requests','incoming'),lambda rows:ids.issubset({row['requestId'] for row in rows if row['canApprove']}))
            rows = {row['requestId']:row for row in rows}
            async def resolve(index, applicant, request):
                row = rows[request['requestId']]
                async def cancel():
                    await asyncio.sleep([0,.002,.01,.03][index % 4])
                    return await applicant.command('space','cancel',row['requestId'],error=None)
                await asyncio.gather(cancel(),a.command('space','approve',row['requestId'],str(row['assignmentVersion']),error=None))
                final = await applicant.wait(('space','refresh',row['requestId']),lambda v:v.get('status') in ('cancelled','joined'),allow_error=True)
                await applicant.stop(crash=True); await applicant.start()
                restored = await applicant.command('space','refresh',row['requestId'])
                assert restored['status']==final['status'], (final,restored)
                return applicant,final['status']
            outcomes = await asyncio.gather(*(resolve(index,applicant,request) for index,(applicant,request) in enumerate(zip(applicants,requests))))
            expected.update(applicant.node for applicant,status in outcomes if status=='joined')
            await asyncio.gather(*(node.wait(('space','show'),lambda v:set(v['members'])==expected) for node in nodes))
            for applicant,status in outcomes:
                space = await applicant.command('space','show')
                if status=='cancelled':
                    assert space['members']==[applicant.node], space
                    assert applicant.node not in expected
                else:
                    await applicant.wait(('space','show'),lambda v:set(v['members'])==expected)
            self.event('concurrent-outcomes',applicants=len(outcomes),joined=sum(status=='joined' for _,status in outcomes),cancelled=sum(status=='cancelled' for _,status in outcomes))
            await asyncio.gather(*(applicant.stop() for applicant in applicants))
        await self.stage('directed-many-applicants-concurrent-cancel-approve-restart',concurrent_applicants)

    async def exercise_directed_ring(self, nodes):
        a,b,c,d,e,f = nodes

        async def routes():
            await asyncio.gather(*(self.routes(source, target, True,
                min((j-i) % len(nodes), (i-j) % len(nodes)))
                for i,source in enumerate(nodes) for j,target in enumerate(nodes) if i != j))
            await self.no_shortcuts(nodes)

        async def close_ring():
            await self.pair(a,f)
            await self.members(nodes)
            await routes()
        await self.stage('directed-ring-all-pairs-shortest-routes', close_ring)

        async def delivery(iteration):
            markers = [(source, nodes[(index+3) % len(nodes)], uuid.uuid4().hex)
                       for index,source in enumerate(nodes)]
            await asyncio.gather(*(source.command('space','preference-set',target.node,
                f'ring-from-{source.name}',json.dumps(marker)) for source,target,marker in markers))
            await self.pressure(nodes, iteration)
            # Every observer sees every origin's write, including across three hops.
            await asyncio.gather(*(observer.wait(('space','preferences',target.node),
                lambda value,source=source,marker=marker:value.get(f'ring-from-{source.name}') == marker)
                for observer in nodes for source,target,marker in markers))
            await self.members(nodes)
            await routes()
        for iteration in range(self.args.rounds):
            await self.stage(f'directed-ring-concurrent-sync-{iteration}', lambda iteration=iteration: delivery(iteration))

        async def reroute():
            await c.stop(crash=True)
            marker = uuid.uuid4().hex
            try:
                await asyncio.gather(
                    a.wait(('control','device','list'), lambda rows:any(row['deviceId']==d.node and
                        row['online'] and row.get('relayPath')==[f.node,e.node,d.node] for row in rows)),
                    d.wait(('control','device','list'), lambda rows:any(row['deviceId']==a.node and
                        row['online'] and row.get('relayPath')==[e.node,f.node,a.node] for row in rows)))
                await a.command('space','preference-set',d.node,'ring-bypass-forward',json.dumps(marker))
                await d.command('space','preference-set',a.node,'ring-bypass-reverse',json.dumps(marker))
                await a.command('space','preference-set',c.node,'ring-offline',json.dumps(marker))
                await asyncio.gather(
                    d.wait(('space','preferences',d.node),lambda value:value.get('ring-bypass-forward')==marker),
                    a.wait(('space','preferences',a.node),lambda value:value.get('ring-bypass-reverse')==marker))
            finally:
                await c.start()
            await c.wait(('space','preferences',c.node),lambda value:value.get('ring-offline')==marker)
            await self.members(nodes)
            await routes()
        await self.stage('directed-ring-crashed-relay-bypass-and-recovery', reroute)

        async def restart():
            await asyncio.gather(*(node.stop(crash=True) for node in nodes))
            for node in reversed(nodes):
                await node.start()
            await self.members(nodes)
            await routes()
            await delivery(self.args.rounds)
        await self.stage('directed-ring-all-killed-reverse-restart-sync', restart)

    async def exercise_departure(self):
        nodes = [await self.node(name) for name in 'ABCD']
        a,b,c,d = nodes
        retained = [a,b,d]
        original = (await a.command('space','show'))['spaceId']
        await self.stage('departure-os-enforces-directed-chain', lambda: asyncio.gather(*(self.verify_directed_policy(s) for s in nodes)))

        async def admit():
            for index in range(1,len(nodes)):
                await self.pair(nodes[index],nodes[index-1])
                await self.join(nodes[index],nodes[index-1],a,nodes[:index+1])
            await asyncio.gather(self.routes(a,d,True,3),self.routes(d,a,True,3))
            if self.args.replicas:
                role=await a.command('control','identity','define','DepartureReplica','view','relay','storage','execute')
                for node in nodes[1:]:
                    await a.command('control','identity','set',node.node,role['roleId'])
                await asyncio.gather(*(node.wait(('control','show'),lambda value,node=node:value['deviceIdentityIds'].get(node.node)==role['roleId']) for node in nodes[1:]))
                chat=await a.command('core','chat','new')
                await d.wait(('core','chat','list'),lambda rows:any(row['id']==chat['chatId'] for row in rows))
        await self.stage('departure-four-node-chain',admit)

        async def views(label):
            await self.members(retained)
            for node in retained:
                space = await node.command('space','show')
                assert space['spaceId']==original, space
                if node == a:
                    await node.wait(('control','audit'),lambda rows:any(row['issuerNodeId']==c.node and row['accepted'] and row['summary']=='Leave Space' for row in rows))
                topology = await node.command('space','topology')
                assert {row['deviceId'] for row in topology['devices']}=={s.node for s in retained},topology
                assert all(c.node not in (edge['firstDeviceId'],edge['secondDeviceId']) for edge in topology['connections']),topology
                (self.directory/f'{label}-{node.name}-topology.json').write_text(json.dumps(topology,indent=2))
            await asyncio.gather(self.routes(a,d,False),self.routes(b,d,False),self.routes(d,a,False),self.routes(d,b,False))
            own = await c.command('space','show')
            assert own['members']==[c.node] and own['spaceId']!=original,own
            state = await c.command('control','show')
            assert state['deviceIdentityIds']=={c.node:'admin'},state
            await self.no_shortcuts(nodes)

        async def leave_online():
            started=time.monotonic()
            await c.command('space','leave',timeout=10)
            self.event('local-exit-returned',seconds=round(time.monotonic()-started,3))
            await views('online-exit')
        await self.stage('departure-reaches-both-neighbors-and-multihop-a',leave_online)

        async def rejoin():
            await self.join(c,b,a,nodes)
            await asyncio.gather(self.routes(a,d,True,3),self.routes(d,a,True,3))
            await c.stop(crash=True)
            await c.start()
            await self.members(nodes)
            await self.routes(a,d,True,3)
        await self.stage('departure-old-receipt-cannot-undo-readmission',rejoin)

        async def leave_offline():
            await asyncio.gather(b.stop(crash=True),d.stop(crash=True))
            started=time.monotonic()
            own=await c.command('space','leave',timeout=10)
            self.event('offline-local-exit-returned',seconds=round(time.monotonic()-started,3))
            await c.stop(crash=True)
            await c.start()
            assert await c.command('space','show')==own
            # No exit receipt can cross the failed B link to A yet.
            assert c.node in (await a.command('space','show'))['members']
            await self.routes(a,c,False)
            await d.start()
            await d.wait(('space','show'),lambda space:c.node not in space['members'])
            assert c.node in (await a.command('space','show'))['members']
            await c.stop(crash=True)
            await c.start()
            await b.start()
            await views('offline-retried-exit')
        await self.stage('departure-offline-neighbors-reboot-durable-retry',leave_offline)

        marker=uuid.uuid4().hex
        partition_chats={}
        async def partition():
            await a.command('space','preference-set',d.node,'departure-forward',json.dumps(marker))
            await d.command('space','preference-set',a.node,'departure-reverse',json.dumps(marker))
            if self.args.replicas:
                partition_chats[a.name]=await a.command('core','chat','new')
                partition_chats[d.name]=await d.command('core','chat','new')
            await c.stop(crash=True)
            await asyncio.gather(b.stop(crash=True),d.stop(crash=True))
            await b.start()
            await d.start()
            await self.members(retained)
            await asyncio.gather(self.routes(a,d,False),self.routes(d,a,False))
        await self.stage('departure-partition-retains-space-and-local-writes',partition)

        async def membership_snapshot():
            state={}
            for node in retained:
                space=await node.command('space','show')
                # A formerly isolated replica may catch up the existing revision.
                # Compare membership content and the full control history instead.
                state[node.name]={
                    'space':{key:value for key,value in space.items() if key!='spaceRevision'},
                    'control':await node.command('control','show'),
                    'incoming':await node.command('space','requests','incoming'),
                    'outgoing':await node.command('space','requests','outgoing'),
                }
            state['audit']=await a.command('control','audit')
            return state

        membership_before=await membership_snapshot()
        (self.directory/'channel-membership-before.json').write_text(json.dumps(membership_before,indent=2))

        async def unchanged_membership(label):
            after=await membership_snapshot()
            (self.directory/f'{label}-membership-after.json').write_text(json.dumps(after,indent=2))
            assert after==membership_before, 'adding a channel changed Space membership, identities, requests or control history'
            await self.no_shortcuts(retained)

        async def bridge():
            # A new physical route; the process sandbox now permits just B.
            self.dial_targets['D']=['B']
            await d.stop(crash=True)
            await d.start()
            await self.verify_directed_policy(d)
            await self.pair(d,b)
            # Pairing reconnects an existing member; no join/approval is issued.
            await self.members(retained)
            await asyncio.gather(self.routes(a,d,True,2),self.routes(d,a,True,2))
            await asyncio.gather(
                d.wait(('space','preferences',d.node),lambda value:value.get('departure-forward')==marker),
                a.wait(('space','preferences',a.node),lambda value:value.get('departure-reverse')==marker))
            if self.args.replicas:
                await asyncio.gather(
                    a.wait(('core','chat','list'),lambda rows:any(row['id']==partition_chats[d.name]['chatId'] for row in rows)),
                    d.wait(('core','chat','list'),lambda rows:any(row['id']==partition_chats[a.name]['chatId'] for row in rows)))
            for node in retained:
                topology=await node.command('space','topology')
                assert {row['deviceId'] for row in topology['devices']}=={s.node for s in retained}
                assert all(c.node not in (edge['firstDeviceId'],edge['secondDeviceId']) for edge in topology['connections'])
                (self.directory/f'bridged-{node.name}-topology.json').write_text(json.dumps(topology,indent=2))
            await unchanged_membership('bridged')
        await self.stage('departure-new-route-recovers-without-joining-again',bridge)

        async def direct_channel():
            # Keep B's route and add A's: both belong to the existing Space.
            self.dial_targets['D']=['A','B']
            await d.stop(crash=True)
            await d.start()
            await self.verify_directed_policy(d)
            await self.pair(d,a)
            await asyncio.gather(self.routes(a,d,True,1),self.routes(d,a,True,1))
            await unchanged_membership('direct')
            # Direct traffic must survive B going away, including a D restart.
            await b.stop(crash=True)
            await d.stop(crash=True)
            await d.start()
            await asyncio.gather(self.routes(a,d,True,1),self.routes(d,a,True,1))
            value=uuid.uuid4().hex
            await asyncio.gather(
                a.command('space','preference-set',d.node,'direct-forward',json.dumps(value)),
                d.command('space','preference-set',a.node,'direct-reverse',json.dumps(value)))
            await asyncio.gather(
                d.wait(('space','preferences',d.node),lambda row:row.get('direct-forward')==value),
                a.wait(('space','preferences',a.node),lambda row:row.get('direct-reverse')==value))
            await b.start()
            await self.members(retained)
            await unchanged_membership('direct-restarted')
        await self.stage('departure-extra-direct-channel-preserves-membership-and-restarts',direct_channel)

    async def exercise_silent_removal(self):
        nodes=[await self.node(name) for name in 'ABCD']
        a,b,c,d=nodes
        retained=[a,b,d]
        original=(await a.command('space','show'))['spaceId']

        async def admit():
            await asyncio.gather(*(self.verify_directed_policy(node) for node in nodes))
            for index in range(1,len(nodes)):
                await self.pair(nodes[index],nodes[index-1])
                await self.join(nodes[index],nodes[index-1],a,nodes[:index+1])
            await asyncio.gather(self.routes(a,d,True,3),self.routes(d,a,True,3))
        await self.stage('silent-removal-four-node-directed-chain',admit)

        async def disappear():
            await c.stop(crash=True)
            self.dial_targets['C']=[]
            await c.start()
            await self.verify_directed_policy(c)
            for peer in await c.command('peers'):
                await c.command('unpair',peer['nodeId'])
            assert await c.command('peers')==[]
            own=await c.command('space','leave')
            assert own['spaceId']!=original and own['members']==[c.node]
            await c.stop(crash=True)
            await asyncio.gather(self.routes(a,d,False),self.routes(d,a,False))
        await self.stage('silent-removal-offline-unpair-and-leave-without-notice',disappear)

        async def bridge():
            self.dial_targets['D']=['B']
            await d.stop(crash=True)
            await d.start()
            await self.verify_directed_policy(d)
            await self.pair(d,b)
            # The former Space still knows C as a member. D adds a channel only.
            for node in retained:
                space=await node.command('space','show')
                assert space['spaceId']==original and set(space['members'])=={s.node for s in nodes},space
            assert not any(row['issuerNodeId']==c.node and row['summary']=='Leave Space'
                for row in await a.command('control','audit'))
            await asyncio.gather(self.routes(a,d,True,2),self.routes(d,a,True,2))
        await self.stage('silent-removal-d-reconnects-b-in-the-original-space',bridge)

        async def removed_views(label):
            await self.members(retained)
            for node in retained:
                space=await node.command('space','show')
                control=await node.command('control','show')
                topology=await node.command('space','topology')
                assert space['spaceId']==original
                assert control['deviceIdentityIds']=={a.node:'admin',b.node:'user',d.node:'user'},control
                assert {row['deviceId'] for row in topology['devices']}=={s.node for s in retained},topology
                assert all(c.node not in (edge['firstDeviceId'],edge['secondDeviceId']) for edge in topology['connections']),topology
                assert all(peer['nodeId']!=c.node for peer in await node.command('peers'))
                (self.directory/f'{label}-{node.name}.json').write_text(json.dumps(
                    {'space':space,'control':control,'topology':topology},indent=2))
            await asyncio.gather(self.routes(a,d,True,2),self.routes(d,a,True,2))
            marker=uuid.uuid4().hex
            await asyncio.gather(
                a.command('space','preference-set',d.node,'removed-forward',json.dumps(marker)),
                d.command('space','preference-set',a.node,'removed-reverse',json.dumps(marker)))
            await asyncio.gather(
                d.wait(('space','preferences',d.node),lambda row:row.get('removed-forward')==marker),
                a.wait(('space','preferences',a.node),lambda row:row.get('removed-reverse')==marker))
            await self.no_shortcuts(retained)

        async def remove():
            await a.command('space','remove',c.node)
            audit=await a.command('control','audit')
            assert any(row['accepted'] and row['issuerNodeId']==a.node and row['summary'].startswith('Remove device ') for row in audit),audit
            (self.directory/'removal-audit.json').write_text(json.dumps(audit,indent=2))
            await removed_views('removed')
        await self.stage('silent-removal-a-removes-c-and-abd-converge',remove)

        async def restart():
            await asyncio.gather(*(node.stop(crash=True) for node in retained))
            for node in reversed(retained):
                await node.start()
            await removed_views('restarted')
        await self.stage('silent-removal-restarts-retain-removal-and-two-way-sync',restart)

    async def exercise_directed_pair(self):
        a,b=[await self.node(name) for name in 'AB']
        nodes=[a,b]
        before={node.name:await node.command('space','show') for node in nodes}

        async def pair_only():
            await asyncio.gather(*(self.verify_directed_policy(node) for node in nodes))
            await self.pair(b,a)
            for node in nodes:
                assert await node.command('space','show')==before[node.name]
                assert await node.command('space','requests','incoming')==[]
                assert await node.command('space','requests','outgoing')==[]
        await self.stage('directed-pair-keeps-independent-spaces-before-admission',pair_only)

        async def shared_views(label):
            await self.members(nodes)
            space=await a.command('space','show')
            await b.wait(('space','show'),lambda row:row==space)
            control=await a.command('control','show')
            assert control['deviceIdentityIds']=={a.node:'admin',b.node:'user'},control
            await b.wait(('control','show'),lambda row:row==control)
            await asyncio.gather(self.routes(a,b,True,1),self.routes(b,a,True,1))
            for node,peer in ((a,b),(b,a)):
                peers=await node.command('peers')
                assert len(peers)==1 and peers[0]['nodeId']==peer.node,peers
                assert peers[0]['outbound']==(node==b) and peers[0]['inbound']==(node==a),peers
                status=await node.wait(('space','status',peer.node),lambda row:row['status']=='Online')
                topology=await node.command('space','topology')
                assert topology['currentDeviceId']==node.node
                assert {row['deviceId'] for row in topology['devices']}=={a.node,b.node}
                assert all(row['online'] for row in topology['devices']),topology
                (self.directory/f'{label}-{node.name}.json').write_text(json.dumps(
                    {'space':await node.command('space','show'),'control':await node.command('control','show'),
                     'peers':peers,'status':status,'topology':topology},indent=2))

        async def admit():
            await self.join(b,a,a,nodes)
            await shared_views('joined')
        await self.stage('directed-pair-shared-space-and-both-online-after-admission',admit)

        async def rename():
            name='Two CLI '+uuid.uuid4().hex
            await a.command('space','rename',name)
            await b.wait(('space','show'),lambda row:row['spaceName']==name)
            await shared_views('renamed')
        await self.stage('directed-pair-space-metadata-syncs',rename)

        async def preferences():
            marker=uuid.uuid4().hex
            await asyncio.gather(
                a.command('space','preference-set',b.node,'pair-forward',json.dumps(marker)),
                b.command('space','preference-set',a.node,'pair-reverse',json.dumps(marker)))
            await asyncio.gather(
                b.wait(('space','preferences',b.node),lambda row:row.get('pair-forward')==marker),
                a.wait(('space','preferences',a.node),lambda row:row.get('pair-reverse')==marker))
        await self.stage('directed-pair-config-syncs-in-both-directions',preferences)

        for offline,online in ((b,a),(a,b)):
            async def restart(offline=offline,online=online):
                await offline.stop(crash=True)
                await self.routes(online,offline,False)
                status=await online.wait(('space','status',offline.node),lambda row:row['status']=='Offline')
                (self.directory/f'offline-{offline.name}-seen-by-{online.name}.json').write_text(json.dumps(status))
                # Connection loss retains both admitted members and queues writes.
                assert set((await online.command('space','show'))['members'])=={a.node,b.node}
                marker=uuid.uuid4().hex
                await online.command('space','preference-set',offline.node,'pair-queued',json.dumps(marker))
                await offline.start()
                await shared_views(f'restarted-{offline.name}')
                await offline.wait(('space','preferences',offline.node),lambda row:row.get('pair-queued')==marker)
                await preferences()
            await self.stage(f'directed-pair-{offline.name}-offline-restart-and-queued-sync',restart)

    async def exercise(self):
        if self.args.focus == 'directed-pair':
            return await self.exercise_directed_pair()
        if self.args.focus == 'silent-removal':
            return await self.exercise_silent_removal()
        if self.args.focus == 'departure':
            return await self.exercise_departure()
        if self.args.focus in ('directed', 'directed-ring', 'departure', 'silent-removal', 'directed-pair'):
            return await self.exercise_directed()
        started = await asyncio.gather(*(self.node(name) for name in 'ABCDEF'), return_exceptions=True)
        for result in started:
            if isinstance(result, Exception):
                raise result
        a, b, c, d, e, f = started
        nodes = [a,b,c,d,e,f]
        async def groups():
            await asyncio.gather(self.pair(b,a), self.pair(e,d))
            await asyncio.gather(self.join(b,a,a,[a,b]), self.join(e,d,d,[d,e]))
            await asyncio.gather(self.pair(c,b), self.pair(f,e))
            await asyncio.gather(self.join(c,b,a,[a,b,c]), self.join(f,e,d,[d,e,f]))
        if self.args.topology == 'groups':
            await self.stage('two-independent-three-node-spaces', groups)
        async def merge():
            if self.args.topology == 'groups':
                await self.pair(d,c)
                await self.join(d,c,a,nodes)
            else:
                for index in range(1,len(nodes)):
                    await self.pair(nodes[index],nodes[index-1])
                    await self.join(nodes[index],nodes[index-1],a,nodes[:index+1])
            await self.routes(a,f,True,5); await self.routes(f,a,True,5)
            await self.no_shortcuts(nodes)
            for s in nodes:
                state = await s.command('control', 'show')
                assert state['deviceIdentityIds'][a.node] == 'admin'
                assert state['deviceIdentityIds'][d.node] == 'user', 'source admin inherited target administration'
            if self.args.replicas:
                role = await a.command('control', 'identity', 'define', 'StressReplica', 'view', 'relay', 'storage', 'execute')
                for s in nodes[1:]:
                    await a.command('control', 'identity', 'set', s.node, role['roleId'])
                await asyncio.gather(*(s.wait(('control','show'),lambda v,s=s:v['deviceIdentityIds'].get(s.node)==role['roleId']) for s in nodes[1:]))
        await self.stage('group-merge-five-hop-chain' if self.args.topology=='groups' else 'sequential-five-hop-joins', merge)
        await self.stage('concurrent-baseline', lambda:self.pressure(nodes,0))
        async def sync_chats():
            chat = await a.command('core','chat','new')
            await f.wait(('core','chat','list'),lambda rows:any(r['id']==chat['chatId'] for r in rows))
            watch = await f.wait(('chat-watch',chat['chatId']),lambda v:bool(v.get('route',{}).get('ownerReachable')),allow_error=True)
            assert watch['route']['ownerNodeId']==a.node and watch['route']['ownerReachable'],watch
        if self.args.replicas:
            await self.stage('five-hop-chat-sync-and-watch',sync_chats,required=False)
        async def bidirectional_preferences():
            marker=uuid.uuid4().hex
            await a.command('space','preference-set',f.node,'forward-probe',json.dumps(marker))
            await f.command('space','preference-set',a.node,'reverse-probe',json.dumps(marker))
            await asyncio.gather(
                f.wait(('space','preferences',f.node),lambda v:v.get('forward-probe')==marker),
                a.wait(('space','preferences',a.node),lambda v:v.get('reverse-probe')==marker))
        await self.stage('bidirectional-five-hop-preferences',bidirectional_preferences,required=False)
        for iteration in range(1,(self.args.rounds if self.args.focus=='all' else 0)+1):
            async def freeze():
                c.pause()
                try:
                    await self.pressure([a,b,d,e,f],iteration)
                    await asyncio.sleep(self.args.stall_seconds)
                finally:c.resume()
                await self.routes(a,f,True,5)
                await self.members(nodes)
            await self.stage(f'round-{iteration}-stalled-relay',freeze)
            async def crash():
                identity,bind=c.node,c.bind
                await c.stop(crash=True)
                async def offline():
                    await asyncio.gather(self.routes(a,f,False),self.routes(f,a,False))
                await self.stage(f'round-{iteration}-partition-detection',offline,required=False)
                await self.pressure([a,b,d,e,f],100+iteration)
                payload={'round':iteration,'queued':True}
                await a.command('space','preference-set',f.node,'offline-queue',json.dumps(payload))
                await c.start(bind)
                assert c.node==identity
                await asyncio.gather(self.routes(a,f,True,5),self.routes(f,a,True,5))
                await self.members(nodes)
                await self.no_shortcuts(nodes)
                async def queued():
                    await f.wait(('space','preferences',f.node),lambda v:v.get('offline-queue')==payload)
                await self.stage(f'round-{iteration}-queued-preferences',queued,required=False)
            await self.stage(f'round-{iteration}-crash-partition-reconnect',crash,required=False)
            if c.process.returncode is not None:
                await c.start()
        async def cancel():
            g=await self.node('G');await self.pair(g,f)
            before=await a.command('space','show')
            req=await g.command('space','join',f.node)
            rows=await a.wait(('space','requests','incoming'),lambda rows:any(r['requestId']==req['requestId'] for r in rows))
            row=next(r for r in rows if r['requestId']==req['requestId'])
            result=await g.command('space','cancel',req['requestId'])
            assert result['status']=='cancelled',result
            await a.command('space','approve',row['requestId'],str(row['assignmentVersion']),error=True)
            assert await a.command('space','show')==before
            await self.members(nodes)
            await g.stop()
        if self.args.focus!='migration':
            await self.stage('six-hop-pending-cancel-stale-approval',cancel,required=False)
        g = self.sessions[-1]
        if g.name == 'G' and g.process.returncode is None:
            for req in await g.command('space','requests','outgoing'):
                if req['status']=='pending':
                    await g.command('space','cancel',req['requestId'])
            await g.stop()
        async def leave():
            old=await f.command('space','show')
            fresh=await f.command('space','leave')
            assert fresh['spaceId']!=old['spaceId'] and fresh['members']==[f.node],fresh
            # Leave is local: the old Space retains its member list and pairings.
            assert (await a.command('space','show'))['spaceId']==old['spaceId']
            assert set((await e.command('space','show'))['members'])=={s.node for s in nodes}
            await self.join(f,e,a,nodes)
            await self.routes(a,f,True,5)
        if self.args.focus!='migration':
            await self.stage('tail-leave-and-rejoin',leave,required=False)
        if (await f.command('space','show'))['spaceId'] != (await a.command('space','show'))['spaceId']:
            async def recover_tail():
                await self.join(f,e,a,nodes)
            await self.stage('tail-rejoin-recovery',recover_tail)
        for bind,label in [('127.0.0.1:0','new-port'),('[::1]:0','ipv4-to-ipv6'),('127.0.0.1:0','ipv6-to-ipv4')]:
            async def migrate(bind=bind):
                previous=c.bind
                await c.stop();await c.start(bind)
                assert c.bind!=previous
                await asyncio.gather(self.routes(a,f,True,5),self.routes(f,a,True,5))
                await self.members(nodes);await self.no_shortcuts(nodes)
                await self.pressure(nodes,200)
                await bidirectional_preferences()
            await self.stage(f'bridge-migration-{label}',migrate,required=False)
            if c.process.returncode is not None:
                await c.start('127.0.0.1:0')
        async def restart():
            await asyncio.gather(*(s.stop(crash=True) for s in nodes))
            for s in reversed(nodes):await s.start()
            await asyncio.gather(self.routes(a,f,True,5),self.routes(f,a,True,5))
            await self.members(nodes);await self.no_shortcuts(nodes)
            await bidirectional_preferences()
        await self.stage('all-processes-killed-reverse-order-restart',restart)
        async def alternate_path():
            await self.pair(f,a)
            await self.routes(a,f,True,1)
            await c.stop(crash=True)
            try:
                async def route():
                    await a.wait(('control','device','list'), lambda rows:any(
                        r['deviceId']==d.node and r['online'] and
                        r.get('relayPath')==[f.node,e.node,d.node] for r in rows))
                async def delivery():
                    marker=uuid.uuid4().hex
                    await a.command('space','preference-set',d.node,'alternate-path',json.dumps(marker))
                    await d.wait(('space','preferences',d.node),lambda v:v.get('alternate-path')==marker)
                passed=await asyncio.gather(
                    self.stage('ring-route-convergence',route,required=False),
                    self.stage('ring-preference-delivery',delivery,required=False))
                assert all(passed), 'ring route/delivery failed; see individual scenario results'
            finally:
                await c.start()
            await self.members(nodes);await self.no_shortcuts(nodes)
        await self.stage('ring-reroutes-around-failed-relay',alternate_path)

    async def run(self):
        failed=None
        try:await self.exercise()
        except Exception:
            failed=traceback.format_exc();(self.directory/'failure.txt').write_text(failed)
            print(failed,flush=True)
        finally:
            cleanup=await asyncio.gather(*(s.stop() for s in self.sessions),return_exceptions=True)
            errors=[str(e) for e in cleanup if isinstance(e,Exception)]
            ordered=sorted(self.latencies)
            report={'passed':failed is None and not errors and all(r['passed'] for r in self.results),'transport':self.args.transport,'rounds':self.args.rounds,'replicas':self.args.replicas,'topology':self.args.topology,'focus':self.args.focus,'operations':self.args.operations,'stall_seconds':self.args.stall_seconds,
                    'commands':len(ordered),'command_counts':self.command_counts,'settle_seconds':self.args.settle_seconds,'command_p50_ms':round(statistics.median(ordered)*1000,2) if ordered else None,
                    'command_p95_ms':round(ordered[min(len(ordered)-1,int(len(ordered)*.95))]*1000,2) if ordered else None,
                    'duration_s':round(time.monotonic()-self.started,2),'scenarios':self.results,'cleanup_errors':errors}
            (self.directory/'report.json').write_text(json.dumps(report,indent=2))
            self.event('summary',**report)
            self.events.close()
            for s in self.sessions:s.log.close();s.stderr.close()
        return not report['passed']


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,default=REPO/'apps/cli/target/debug/operit2')
    parser.add_argument('--transport',choices=['tcp','http','ws','mixed'],default='tcp')
    parser.add_argument('--topology',choices=['groups','chain'],default='groups')
    parser.add_argument('--focus',choices=['all','migration','lifecycle','directed','directed-ring','departure','silent-removal','directed-pair'],default='all')
    parser.add_argument('--settle-seconds',type=float,default=45,help='maximum convergence time for each assertion')
    parser.add_argument('--replicas',action='store_true',help='also grant business storage and test chat replication')
    parser.add_argument('--stall-seconds',type=float,default=3,help='SIGSTOP silence per relay stall')
    parser.add_argument('--rounds',type=int,default=3)
    parser.add_argument('--operations',type=int,default=12,help='writes/reads per node per pressure burst')
    args=parser.parse_args()
    if args.focus in ('directed', 'directed-ring', 'departure', 'silent-removal', 'directed-pair'):
        if sys.platform != 'darwin' or not Path('/usr/bin/sandbox-exec').exists():
            parser.error('directed focuses require macOS sandbox-exec for real per-process network restrictions')
        args.topology = 'ring' if args.focus == 'directed-ring' else 'chain'
    assert args.rounds>0 and args.operations>0
    raise SystemExit(asyncio.run(Run(args).run()))


if __name__=='__main__':main()
