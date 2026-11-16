"""Measure real CLI sync latency with isolated profiles and TCP/HTTP/WS traffic.

Usage: python apps/cli/tests/sync_timing_session.py --binary PATH
Artifacts include phase timings, process CPU/RSS snapshots, and application logs.
File visibility uses 20 ms polling; chat/preferences use the shared 250 ms poll.
"""
import argparse, asyncio, hashlib, json, os, subprocess, time, types
from pathlib import Path
from space_stress_session import Run
from storage_sync_session import identity_root, inspect_nodes

async def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--transport', choices=['tcp', 'http', 'ws'], default='tcp')
    parser.add_argument('--idle-seconds', type=float, default=70)
    parser.add_argument('--settle-seconds', type=float, default=120)
    args = parser.parse_args()
    if args.idle_seconds < 0 or args.settle_seconds <= 0:
        parser.error('idle seconds must be nonnegative and settle seconds must be positive')
    run = Run(types.SimpleNamespace(binary=args.binary, transport=args.transport, focus='all', settle_seconds=args.settle_seconds, operations=1))
    metrics = []
    async def measure(name, fn, **meta):
        run.event('phase-start', name=name, epoch=time.time(), **meta)
        start = time.monotonic()
        result = await fn()
        row = dict(name=name, seconds=round(time.monotonic()-start,6), **meta)
        metrics.append(row); run.event('measurement', epoch=time.time(), **row)
        return result
    async def converge(path, digest):
        timeout = args.settle_seconds
        start=time.monotonic()
        while time.monotonic()-start < timeout:
            if path.is_file() and hashlib.sha256(path.read_bytes()).hexdigest()==digest: return
            await asyncio.sleep(.02)
        raise AssertionError(f'File convergence timeout: {path}')
    async def idle(seconds):
        before = subprocess.check_output(['ps','-o','pid=,time=,rss=','-p',','.join(str(s.process.pid) for s in run.sessions)]).decode()
        await asyncio.sleep(seconds)
        after = subprocess.check_output(['ps','-o','pid=,time=,rss=','-p',','.join(str(s.process.pid) for s in run.sessions)]).decode()
        run.event('idle-resources', seconds=seconds, before=before, after=after)
    try:
        a=await measure('startup_A',lambda:run.node('A'))
        b=await measure('startup_B',lambda:run.node('B'))
        seed=await a.command('core','chat','new')
        await measure('pairing',lambda:run.pair(b,a))
        await measure('join',lambda:run.join(b,a,a,[a,b]))
        await measure('initial_chat_visible',lambda:b.wait(('core','chat','list'),lambda rows:any(r['id']==seed['chatId'] for r in rows)))
        await measure('settle',lambda:asyncio.gather(a.command('space','sync',timeout=120),b.command('space','sync',timeout=120)))
        for i in range(3):
            for method in ('syncClockDigest','syncClock','syncConsistency'):
                result=await measure(f'local_{method}_{i}',lambda m=method:a.command('call','application',m,'null',timeout=120))
                if isinstance(result,dict) and result.get('errors'): raise AssertionError(result['errors'])
        for i in range(5):
            await measure(f'unchanged_sync_{i}',lambda:a.command('space','sync',timeout=120))
        for source,target in ((a,b),(b,a)):
            async def preferences(source=source,target=target):
                await source.command('core','prefs','media-history','31' if source==a else '33','37' if source==a else '39')
                await target.wait(('core','prefs','show'),lambda v:v['maxImageHistoryUserTurns']==(31 if source==a else 33))
            await measure(f'preference_{source.name}_to_{target.name}',preferences)
            async def chat(source=source,target=target):
                value=await source.command('core','chat','new')
                await target.wait(('core','chat','list'),lambda rows:any(r['id']==value['chatId'] for r in rows))
            await measure(f'chat_{source.name}_to_{target.name}',chat)
        wa=identity_root(a,'workspaceRoot')/'timing-audit'; wb=identity_root(b,'workspaceRoot')/'timing-audit'
        wa.mkdir(parents=True,exist_ok=True)
        async def file_case(name,count,size,force):
            payloads=[os.urandom(size) for _ in range(count)]
            expected=[hashlib.sha256(x).hexdigest() for x in payloads]
            start=time.monotonic()
            for n,payload in enumerate(payloads): (wa/f'{name}-{n}.bin').write_bytes(payload)
            run.event('files-written',name=name,write_seconds=time.monotonic()-start)
            if force: await a.command('space','sync',timeout=120)
            await asyncio.gather(*(converge(wb/f'{name}-{n}.bin',digest) for n,digest in enumerate(expected)))
        await measure('file_4MiB_forced',lambda:file_case('single',1,4*1024*1024,True),bytes=4*1024*1024)
        await measure('files_16x256KiB_forced',lambda:file_case('batch',16,256*1024,True),bytes=4*1024*1024)
        await measure('file_1KiB_auto_active',lambda:file_case('active',1,1024,False),bytes=1024)
        for i in range(3):
            await measure(f'unchanged_sync_with_files_{i}',lambda:a.command('space','sync',timeout=120))
        async def offline():
            await b.stop(crash=True)
            payload=os.urandom(256*1024); (wa/'offline.bin').write_bytes(payload)
            await a.command('space','sync',error=None,timeout=120)
            await b.start()
            await converge(wb/'offline.bin',hashlib.sha256(payload).hexdigest())
        await measure('offline_reconnect_256KiB',offline)
        await measure(f'idle_{args.idle_seconds:g}s',lambda:idle(args.idle_seconds))
        await measure('file_1KiB_auto_after_idle',lambda:file_case('idle',1,1024,False),bytes=1024)
        await measure('final_drain',lambda:asyncio.gather(a.command('space','sync',timeout=120),b.command('space','sync',timeout=120)))
        for s in (a,b):
            report=await s.command('call','application','syncConsistency','null',timeout=120)
            assert not report['errors'] and not report['pendingOwners'],report
            run.event('final-consistency',node=s.name,owners=len(report['manifests']),skipped=report.get('skipped'))
        run.event('ALL-PASS')
    except Exception as error:
        run.event('FAIL',error=repr(error))
        await inspect_nodes(run)
        raise
    finally:
        (run.directory/'timings.json').write_text(json.dumps(metrics,indent=2))
        for s in reversed(run.sessions): await s.stop(crash=True)
        print('ARTIFACT_DIRECTORY='+str(run.directory),flush=True)

asyncio.run(main())
