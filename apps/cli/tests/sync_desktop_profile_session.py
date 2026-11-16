"""Clone the active desktop profile and synchronize it with a fresh, localhost-only CLI."""
import argparse, asyncio, ctypes, hashlib, json, os, socket, sqlite3, subprocess, sys, time, types
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from space_stress_session import Run, Session, space_call
from storage_sync_session import identity_root

os.umask(0o077)
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary',type=Path,required=True)
parser.add_argument('--bootstrap',type=Path,default=Path.home()/'Library/Application Support/Operit2/client/link/local_runtime_storage.json')
args=parser.parse_args()
BOOTSTRAP=args.bootstrap
BINARY=args.binary.resolve()

def inventory(root):
    return {str(p.relative_to(root)):(p.stat().st_size,p.stat().st_mtime_ns)
            for p in root.rglob('*') if p.is_file() and not p.is_symlink()}

def summary(root):
    p=root/'data/database/operit2.sqlite'
    with sqlite3.connect(p.as_uri()+'?mode=ro',uri=True) as connection:
        return {t:connection.execute(f'SELECT COUNT(*) FROM "{t}"').fetchone()[0]
                for t in ['chats','messages','message_parts','message_variants']}

def reserve():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1',0));return f'127.0.0.1:{sock.getsockname()[1]}'

class ProcessMemory:
    """Kernel lifetime physical-footprint high water mark plus 100 ms RSS samples."""
    _fields = "user_time system_time pkg_idle_wkups interrupt_wkups pageins wired_size resident_size phys_footprint proc_start_abstime proc_exit_abstime child_user_time child_system_time child_pkg_idle_wkups child_interrupt_wkups child_pageins child_elapsed_abstime diskio_bytesread diskio_byteswritten cpu_time_qos_default cpu_time_qos_maintenance cpu_time_qos_background cpu_time_qos_utility cpu_time_qos_legacy cpu_time_qos_user_initiated cpu_time_qos_user_interactive billed_system_time serviced_system_time logical_writes lifetime_max_phys_footprint instructions cycles billed_energy serviced_energy interval_max_phys_footprint runnable_time".split()
    class Usage(ctypes.Structure):
        pass
    Usage._fields_ = [('uuid', ctypes.c_uint8 * 16)] + [(name, ctypes.c_uint64) for name in _fields]

    def __init__(self, sessions):
        self.sessions, self.peaks = sessions, {}
        self.started, self.samples = time.monotonic(), []
        self.lib = ctypes.CDLL('/usr/lib/libproc.dylib')
        self.lib.proc_pid_rusage.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_void_p]
        self.lib.proc_pid_rusage.restype = ctypes.c_int

    def snapshot(self):
        for node in self.sessions:
            if node.process is None or node.process.returncode is not None: continue
            usage = self.Usage()
            if self.lib.proc_pid_rusage(node.process.pid, 4, ctypes.byref(usage)) != 0: continue
            self.samples.append({'seconds':round(time.monotonic()-self.started,3), 'node':node.name,
                'rss':usage.resident_size,'physical':usage.phys_footprint,'peak_physical':usage.lifetime_max_phys_footprint})
            peak = self.peaks.setdefault(node.name, {})
            peak['current_physical_bytes'] = usage.phys_footprint
            for name, value in [('peak_physical_bytes', usage.lifetime_max_phys_footprint), ('sampled_peak_rss_bytes', usage.resident_size)]:
                peak[name] = max(peak.get(name, 0), value)
        return {node: values.copy() for node, values in self.peaks.items()}

    async def watch(self):
        while True:
            self.snapshot()
            await asyncio.sleep(0.1)

async def call(node,method,**args):
    return await node.command('call','application',method,json.dumps(args),timeout=1800)

async def main():
    config=json.loads(BOOTSTRAP.read_text());identity=config['activeIdentityId']
    source_runtime=Path(config['runtimeRoot'])/'identities'/identity
    source_workspace=Path(config['workspaceRoot'])/'identities'/identity
    initial_runtime,initial_workspace=inventory(source_runtime),inventory(source_workspace)
    run=Run(types.SimpleNamespace(binary=BINARY,transport='tcp',focus='all',settle_seconds=1800,operations=1))
    os.chmod(run.directory,0o700)
    a,b=Session(run,'A','tcp'),Session(run,'B','tcp');run.sessions.extend([a,b])
    a.bind,b.bind=reserve(),reserve()
    policy=run.directory/'localhost-only.sb'
    policy.write_text('\n'.join(['(version 1)','(allow default)',
        '(deny network-outbound (remote tcp))','(deny network-outbound (remote udp))',
        *[f'(allow network-outbound (remote tcp "localhost:{s.bind.rsplit(":",1)[1]}"))' for s in (a,b)]])+'\n')
    run.command_prefix=lambda session:['/usr/bin/sandbox-exec','-f',str(policy)]
    metrics={}
    memory=ProcessMemory(run.sessions)
    memory_task=asyncio.create_task(memory.watch())
    try:
        start=time.monotonic()
        for source,destination in [(source_runtime,a.directory/'runtime/identities'/identity),
                                    (source_workspace,a.directory/'workspaces/identities'/identity)]:
            destination.mkdir(parents=True,exist_ok=True)
            subprocess.run(['/bin/cp','-cR',str(source)+'/.',str(destination)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
        storage=json.loads((a.config/'storage.json').read_text())
        storage.update(activeIdentityId=identity,identities=[{'id':identity,'name':'Desktop data copy','createdAt':0}])
        (a.config/'storage.json').write_text(json.dumps(storage))
        # Export only the existing preference decryption key to the isolated file host.
        # Capture it in memory and never put the secret in arguments, logs or stdout.
        key=subprocess.run(['/usr/bin/security','find-generic-password','-s','Operit2.HostSecretStore',
            '-a','operit.preferences.encryption_key.v1','-w'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        assert key.returncode==0, 'Existing desktop preference decryption key unavailable'
        if key.returncode==0:
            secret=a.directory/'runtime/identities'/identity/'secrets/operit.preferences.encryption_key.v1'
            content=key.stdout.rstrip(b'\n')
            # security prints non-printable Keychain data as hex; the host reads raw bytes.
            try:
                decoded=json.loads(content)
            except (ValueError,UnicodeDecodeError):
                content=bytes.fromhex(content.decode('ascii'))
                decoded=json.loads(content)
            assert isinstance(decoded,dict) and 'format' in decoded and 'key' in decoded, 'Unexpected preference key envelope'
            secret.parent.mkdir(mode=0o700,exist_ok=True);secret.write_bytes(content);secret.chmod(0o600)
        source_summary=summary(a.directory/'runtime/identities'/identity)
        metrics.update(source_counts=source_summary,source_files=len(initial_runtime),source_bytes=sum(x[0] for x in initial_runtime.values()),
            workspace_files=len(initial_workspace),workspace_bytes=sum(x[0] for x in initial_workspace.values()),
            clone_seconds=time.monotonic()-start,key_available=key.returncode==0)
        run.event('real-clone-ready',**metrics)
        start=time.monotonic();await a.start();await b.start();metrics['startup_seconds']=time.monotonic()-start
        start=time.monotonic()
        before=await call(a,'syncConsistency')
        metrics['source_validation_seconds']=time.monotonic()-start
        assert not before['errors'], 'Source copy has consistency errors before pairing'
        speech_digests=lambda report:{m['owner']['path']:m['digest'] for m in report['manifests']
            if m['owner']['path'].endswith(('tts_configs.preferences.json','stt_configs.preferences.json'))}
        initial_speech=speech_digests(before)
        assert len(initial_speech)==2, 'Missing speech preference owners in source'
        # Detach only the copied device from the production Space before the isolated pairing.
        await a.command('space','leave',timeout=1800)
        run.event('isolated-space-ready')
        metrics['memory_before_pairing']=memory.snapshot()
        start=time.monotonic();await run.pair(b,a);metrics['pair_seconds']=time.monotonic()-start
        start=time.monotonic()
        await run.join(b,a,a,[a,b]);metrics['join_seconds']=time.monotonic()-start
        run.event('real-join-complete',seconds=metrics['join_seconds'])
        start=time.monotonic()
        await asyncio.gather(a.command('space','sync',timeout=1800),b.command('space','sync',timeout=1800))
        metrics['drain_after_join_seconds']=time.monotonic()-start
        metrics['join_and_drain_seconds']=metrics['join_seconds']+metrics['drain_after_join_seconds']
        metrics['validation_join_and_drain_seconds']=metrics['source_validation_seconds']+metrics['join_and_drain_seconds']
        metrics['memory_at_drain']=memory.snapshot()
        metrics['peak_increase_over_pairing_baseline_bytes']={name:max(0,peak['peak_physical_bytes']-metrics['memory_before_pairing'][name]['current_physical_bytes']) for name,peak in metrics['memory_at_drain'].items()}
        metrics['performance_goals']={'sync_limit_seconds':15, 'per_process_physical_limit_bytes':64*1024*1024,
            'join_and_drain_met':metrics['join_and_drain_seconds']<=15,
            'sync_met':metrics['validation_join_and_drain_seconds']<=15,
            'memory_met':set(metrics['memory_at_drain'])=={'A','B'} and all(peak['peak_physical_bytes']<=64*1024*1024 for peak in metrics['memory_at_drain'].values())}
        run.event('real-sync-drained',**metrics)
        start=time.monotonic()
        left,right=await asyncio.gather(call(a,'syncConsistency'),call(b,'syncConsistency'))
        for report in (left,right):
            assert not report['errors'] and not report['pendingOwners'] and not report['skipped'], 'Consistency contains errors/pending/skipped; inspect private artifact'
        content=lambda report:{json.dumps(m['owner'],sort_keys=True):m['digest'] for m in report['manifests']}
        assert content(left)==content(right),'Materialized owner digests differ'
        assert speech_digests(left)==initial_speech, 'Source speech preferences changed during empty-peer sync'
        metrics['speech_preference_owners_preserved']=len(initial_speech)
        metrics['target_counts']=summary(identity_root(b,'runtimeRoot'))
        assert metrics['target_counts']==source_summary,'SQL row counts differ'
        assert summary(identity_root(a,'runtimeRoot'))==source_summary,'Source copy business row counts changed'
        # Verify received blob bytes independently of the manifest comparison.
        blobs=list((identity_root(b,'runtimeRoot')/'sync/blobs').glob('*'))
        for blob in blobs:
            digest=hashlib.sha256()
            with blob.open('rb') as stream:
                for chunk in iter(lambda:stream.read(1024*1024),b''):digest.update(chunk)
            assert digest.hexdigest()==blob.name,'Received blob hash differs'
        metrics['verified_blob_files']=len(blobs)
        metrics['verified_blob_bytes']=sum(blob.stat().st_size for blob in blobs)
        metrics['owners']=len(left['manifests']);metrics['verification_seconds']=time.monotonic()-start
        metrics['production_unchanged']=(inventory(source_runtime)==initial_runtime and inventory(source_workspace)==initial_workspace)
        assert metrics['production_unchanged'],'Production profile metadata changed during run'
        metrics['memory_after_verification']=memory.snapshot()
        metrics['performance_goals']['memory_met']=set(metrics['memory_after_verification'])=={'A','B'} and all(peak['peak_physical_bytes']<=64*1024*1024 for peak in metrics['memory_after_verification'].values())
        (run.directory/'real-results.json').write_text(json.dumps(metrics,indent=2))
        run.event('ALL-PASS',**metrics)
    except Exception as error:
        run.event('FAIL',kind=type(error).__name__,reason=str(error)[:500])
        raise
    finally:
        (run.directory/'memory.jsonl').write_text('\n'.join(json.dumps(sample) for sample in memory.samples)+'\n')
        memory_task.cancel()
        await asyncio.gather(memory_task,return_exceptions=True)
        for node in reversed(run.sessions):await node.stop(crash=True)
        print('ARTIFACT_DIRECTORY='+str(run.directory),flush=True)

if __name__=='__main__':
    asyncio.run(main())
