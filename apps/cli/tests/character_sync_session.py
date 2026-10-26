#!/usr/bin/env python3
"""Run actual character/plugin/config synchronization in isolated TCP CLI processes.

All live writes use public application/plugin APIs. No mocked host, network or DB.
An optional released-v1 profile is copied, never opened in place, for upgrade tests.
"""
import argparse
import asyncio
import hashlib
import json
import shutil
import sys
import time
import traceback
import types
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / 'apps/cli/tests'))
from space_stress_session import Run, Session
from preference_sync_session import create, converge


async def core(node, *arguments, error=False):
    """Invoke the real Core dispatcher rather than the link-session command grammar."""
    return await node.command('core', *arguments, error=error)


async def card(node, identity, expected):
    """Wait for the requested fields, not merely for the record to exist."""
    return await node.wait(('core', 'character', 'show', identity),
        lambda value: isinstance(value, dict) and value.get('id') == identity
        and all(value.get(key) == field for key, field in expected.items()),
        timeout=30, allow_error=True)


async def new_card(node, name, **values):
    """Create a card through the packaged plugin's actual command handler."""
    result = await core(node, 'character', 'create', '--record',
                        json.dumps({'name': name, **values}, ensure_ascii=False))
    return await card(node, result['id'], {'name': name})


async def patch_card(node, identity, **changes):
    """Apply one atomic record patch through the plugin owner."""
    result = await core(node, 'character', 'update', identity, '--patch',
                        json.dumps(changes, ensure_ascii=False))
    return result['character']


async def directory(a, b):
    """Require both independently materialized character directories to converge."""
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        left, right = await asyncio.gather(core(a, 'character', 'list'), core(b, 'character', 'list'))
        if sorted(left, key=lambda item: item['id']) == sorted(right, key=lambda item: item['id']):
            return left
        await asyncio.sleep(.25)
    left_by_id = {item['id']: item for item in left}
    right_by_id = {item['id']: item for item in right}
    differences = []
    for identity in sorted(left_by_id.keys() | right_by_id.keys()):
        if identity not in left_by_id or identity not in right_by_id:
            differences.append({'id': identity, 'presentA': identity in left_by_id, 'presentB': identity in right_by_id})
            continue
        fields = {key: {'A': left_by_id[identity].get(key), 'B': right_by_id[identity].get(key)}
                  for key in left_by_id[identity].keys() | right_by_id[identity].keys()
                  if left_by_id[identity].get(key) != right_by_id[identity].get(key)}
        if fields:
            differences.append({'id': identity, 'name': left_by_id[identity]['name'], 'fields': fields})
    raise AssertionError({'kind': 'character-directory-divergence', 'differences': differences})


async def execute_profile(node, identity, provider, tts, error=False):
    """Resolve an execution profile without making a model-provider network request."""
    arguments = {
        'packageName': 'com.operit.character_cards', 'methodName': 'chat.configuration.resolve',
        'payload': {'purpose': 'execution', 'chatId': None,
                    'chatExtension': {'version': 1, 'selection': 'card:' + identity},
                    'participantId': None, 'promptFunctionType': 'CHAT',
                    'defaultModelBinding': {'providerId': provider, 'modelId': 'sync-model'},
                    'defaultTtsConfigId': tts},
    }
    return await node.command('call', 'application.packageManager', 'invokeToolPkgPublicApi',
                              json.dumps(arguments), error=error, timeout=25)


def profile_digest(directory):
    """Fingerprint every source-profile file to prove upgrade testing never edits it."""
    digest = hashlib.sha256()
    for path in sorted(directory.rglob('*')):
        if path.is_file():
            digest.update(path.relative_to(directory).as_posix().encode('utf-8'))
            digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


async def live_lifecycle(run, a, b, canonical_provider, canonical_tts, exact_provider, result):
    """Test live records, exact execution binding, restart, deletion and leave/rejoin."""
    fresh = await new_card(a, 'After join', description='created on A')
    await card(b, fresh['id'], {'description': 'created on A'})
    await patch_card(a, fresh['id'], description='updated on A', characterSetting='A setting')
    await card(b, fresh['id'], {'description': 'updated on A', 'characterSetting': 'A setting'})
    await patch_card(b, fresh['id'], description='updated on B')
    await card(a, fresh['id'], {'description': 'updated on B', 'characterSetting': 'A setting'})
    result('live-create-and-bidirectional-same-uuid-updates', id=fresh['id'])
    invalid = await core(a, 'character', 'update', fresh['id'], '--patch',
        json.dumps({'chatModelBindingMode': 'FIXED_MODEL', 'chatProviderId': None,
                    'chatModelId': 'sync-model'}), error=True)
    await card(a, fresh['id'], {'chatModelBindingMode': 'FOLLOW_GLOBAL', 'chatProviderId': None})
    result('missing-provider-rejected-without-changing-card', error=invalid['error'])
    exact_b = exact_provider
    changed = await patch_card(b, fresh['id'], chatModelBindingMode='FIXED_MODEL',
                              chatProviderId=exact_b, chatModelId='sync-model', ttsConfigId=canonical_tts)
    await card(a, fresh['id'], {'chatModelBindingMode': 'FIXED_MODEL',
               'chatProviderId': exact_b, 'chatModelId': 'sync-model'})
    resolved = await execute_profile(a, fresh['id'], canonical_provider, canonical_tts)
    assert resolved['profile']['modelBinding']['providerId'] == exact_b, resolved
    result('atomic-provider-model-pair-update-and-execution', card=changed, resolved=resolved['profile']['modelBinding'])
    retained = await new_card(a, 'Retained bound card',
        chatModelBindingMode='FIXED_MODEL', chatProviderId=exact_b,
        chatModelId='sync-model', ttsConfigId=canonical_tts)
    await card(b, retained['id'], {'chatProviderId': exact_b, 'ttsConfigId': canonical_tts})
    await b.stop()
    await patch_card(a, fresh['id'], description='written while B was offline')
    await b.start()
    await card(b, fresh['id'], {'description': 'written while B was offline', 'chatProviderId': exact_b})
    result('restart-and-offline-catch-up-preserve-binding', id=fresh['id'])
    await core(b, 'character', 'delete', fresh['id'])
    after_delete = await directory(a, b)
    assert fresh['id'] not in {item['id'] for item in after_delete}
    result('deletion-converges-bidirectionally', deletedId=fresh['id'])
    before_rejoin = {item['id'] for item in after_delete}
    await b.command('space', 'leave')
    await run.join(b, a, a, [a, b])
    for _ in range(8):
        after_rejoin = await directory(a, b)
        assert before_rejoin == {item['id'] for item in after_rejoin}, after_rejoin
        for node in [a, b]:
            await card(node, retained['id'], {'chatProviderId': exact_b, 'ttsConfigId': canonical_tts})
        await asyncio.sleep(.5)
    for node in [a, b]:
        resolved = await execute_profile(node, retained['id'], canonical_provider, canonical_tts)
        assert resolved['profile']['modelBinding'] == {'providerId': exact_b, 'modelId': 'sync-model'}, resolved
    result('leave-rejoin-retains-uuid-set-and-does-not-resurrect-deletion', ids=sorted(before_rejoin),
           retainedBinding=resolved['profile']['modelBinding'], observationSeconds=4)


async def legacy_upgrade(run, legacy_profile, result):
    """Upgrade only a copied released profile and require a second clean startup."""
    await asyncio.gather(*(node.stop() for node in run.sessions))
    legacy = Session(run, 'legacy-copy', 'tcp')
    run.sessions.append(legacy)
    before = profile_digest(legacy_profile)
    config = json.loads((legacy_profile / 'config/storage.json').read_text(encoding='utf-8'))
    shutil.copytree(legacy_profile / 'runtime', legacy.directory / 'runtime')
    config.update(runtimeRoot=str(legacy.directory / 'runtime'),
                  workspaceRoot=str(legacy.directory / 'workspaces'))
    (legacy.config / 'storage.json').write_text(json.dumps(config), encoding='utf-8')
    await legacy.start()
    upgraded = await core(legacy, 'character', 'list')
    assert all('chatProviderId' in item for item in upgraded), upgraded
    await legacy.stop()
    await legacy.start()
    assert await core(legacy, 'character', 'list') == upgraded
    assert profile_digest(legacy_profile) == before, 'legacy source profile was modified'
    result('released-v1-profile-upgrades-and-restarts', source=str(legacy_profile), cards=upgraded)


async def exercise(binary, legacy_profile, focus):
    """Run join, alias/reference, exact binding, lifecycle and migration scenarios."""
    run = Run(types.SimpleNamespace(binary=binary, transport='tcp', focus='all',
                                   settle_seconds=30, operations=1))
    report = {'binary': str(binary.resolve()), 'sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'artifacts': str(run.directory), 'transport': 'tcp', 'physical_devices': False, 'focus': focus,
              'isolation': 'separate CLI processes, identities and data directories; loopback TCP',
              'cases': []}

    def result(name, **observed):
        """Persist actual evidence and the precise successful assertion name."""
        report['cases'].append({'case': name, 'passed': True, 'observed': observed})
        run.event('PASS', case=name)
        (run.directory / 'character-report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')

    try:
        if focus == 'legacy':
            assert legacy_profile is not None, '--focus legacy requires --legacy-profile'
            await legacy_upgrade(run, legacy_profile, result)
        elif focus == 'lifecycle':
            a, b = await run.node('A'), await run.node('B')
            result('two-real-processes', pids=[a.process.pid, b.process.pid])
            await run.pair(b, a)
            await run.join(b, a, a, [a, b])
            result('real-authenticated-tcp-pair-and-space-admission', nodeIds=[a.node, b.node])
            provider = await create(a, 'provider', 'Live provider', 0)
            other = await create(b, 'provider', 'Other provider', 1)
            tts = await create(a, 'tts', 'Live TTS', 0)
            await converge(a, b, 'provider', timeout=30)
            await converge(a, b, 'tts', timeout=30)
            result('live-configuration-catalogs-converge', providerId=provider['id'], otherProviderId=other['id'], ttsId=tts['id'])
            await live_lifecycle(run, a, b, provider['id'], tts['id'], other['id'], result)
            if legacy_profile is not None:
                await legacy_upgrade(run, legacy_profile, result)
        else:
            a, b = await run.node('A'), await run.node('B')
            original = {}
            for node, variation in [(a, 1), (b, 2)]:
                original[node.name] = {
                    'equal_provider': await create(node, 'provider', 'Equal provider', 0),
                    'collision_provider': await create(node, 'provider', 'Collision provider', variation),
                    'tts': await create(node, 'tts', 'Equal TTS', 0),
                    'theme': await create(node, 'theme', 'Equal theme', 0),
                }
                owned = original[node.name]
                owned['bound'] = await new_card(node, 'Bound-' + node.name,
                    chatModelBindingMode='FIXED_MODEL', chatModelId='sync-model',
                    chatProviderId=owned['equal_provider']['id'], ttsConfigId=owned['tts']['id'],
                    themeConfigId=owned['theme']['id'])
                owned['collision'] = await new_card(node, 'Collision-bound-' + node.name,
                    chatModelBindingMode='FIXED_MODEL', chatModelId='sync-model',
                    chatProviderId=owned['collision_provider']['id'], ttsConfigId=owned['tts']['id'])
                if focus == 'all':
                    owned['same_card'] = await new_card(node, 'Same card', description='same exact content')
                    owned['different_card'] = await new_card(node, 'Conflicting card', description='source ' + node.name)
            result('two-real-processes-with-independent-prejoin-data', pids=[a.process.pid, b.process.pid])
            await run.pair(b, a)
            await run.join(b, a, a, [a, b])
            result('real-authenticated-tcp-pair-and-space-admission', nodeIds=[a.node, b.node])
            providers, _ = await converge(a, b, 'provider', timeout=30)
            tts, _ = await converge(a, b, 'tts', timeout=30)
            themes, _ = await converge(a, b, 'theme', timeout=30)
            equal = [item for item in providers if item['name'] == 'Equal provider']
            assert len(equal) == 1, equal
            canonical_provider = equal[0]['id']
            canonical_tts = next(item['id'] for item in tts if item['name'] == 'Equal TTS')
            canonical_theme = next(item['id'] for item in themes if item['name'] == 'Equal theme')
            result('same-content-preference-uuid-aliases', original=original,
                   retained={'provider': canonical_provider, 'tts': canonical_tts, 'theme': canonical_theme})
            cards = await directory(a, b)
            if focus == 'all':
                identical = [item for item in cards if item['name'] == 'Same card']
                conflicting = [item for item in cards if item['name'].startswith('Conflicting card')]
                assert len(identical) == 1, identical
                assert len(conflicting) == 2 and len({item['name'] for item in conflicting}) == 2, conflicting
                assert len([item for item in cards if item['id'] == 'default']) == 1, cards
                result('card-content-dedup-name-collision-and-default-singleton',
                       identical=identical, conflicting=conflicting, count=len(cards))
            for node_name, owned in original.items():
                for node in [a, b]:
                    bound = await card(node, owned['bound']['id'], {
                        'chatProviderId': canonical_provider, 'chatModelId': 'sync-model',
                        'ttsConfigId': canonical_tts, 'themeConfigId': canonical_theme})
                    resolved = await execute_profile(node, bound['id'], canonical_provider, canonical_tts)
                    assert resolved['profile']['modelBinding'] == {'providerId': canonical_provider, 'modelId': 'sync-model'}, resolved
                result('prejoin-card-references-remapped-' + node_name,
                       id=owned['bound']['id'], providerId=canonical_provider, ttsId=canonical_tts, themeId=canonical_theme)
            for node_name, owned in original.items():
                provider = next(item for item in providers if item['id'] == owned['collision_provider']['id'])
                for node in [a, b]:
                    bound = await card(node, owned['collision']['id'], {
                        'chatProviderId': provider['id'], 'chatModelId': 'sync-model'})
                    resolved = await execute_profile(node, bound['id'], canonical_provider, canonical_tts)
                    assert resolved['profile']['modelBinding'] == {'providerId': provider['id'], 'modelId': 'sync-model'}, resolved
                result('same-model-id-stays-with-exact-provider-' + node_name,
                       cardId=owned['collision']['id'], provider=provider)
            await live_lifecycle(run, a, b, canonical_provider, canonical_tts,
                                 original['B']['collision_provider']['id'], result)
            if legacy_profile is not None:
                await legacy_upgrade(run, legacy_profile, result)
        report['passed'] = True
    except Exception as error:
        report.update(passed=False, error=str(error), traceback=traceback.format_exc())
        run.event('FAIL', error=str(error)[:1000])
        if focus == 'bindings':
            aliases = next((case['observed']['retained'] for case in report['cases']
                            if case['case'] == 'same-content-preference-uuid-aliases'), None)
            if aliases is not None:
                report['binding_diagnostics'] = []
                for node in run.sessions:
                    for owner in original.values():
                        identity = owner['bound']['id']
                        try:
                            stored = await core(node, 'character', 'show', identity)
                            resolved = await execute_profile(node, identity, aliases['provider'], aliases['tts'], error=None)
                            report['binding_diagnostics'].append({'node': node.name, 'card': stored, 'execution': resolved})
                        except Exception as diagnostic_error:
                            report['binding_diagnostics'].append({'node': node.name, 'id': identity,
                                                                  'diagnostic_error': str(diagnostic_error)})
    finally:
        cleanup = await asyncio.gather(*(node.stop() for node in run.sessions), return_exceptions=True)
        report['cleanup_errors'] = [str(item) for item in cleanup if isinstance(item, Exception)]
        report['passed'] = report['passed'] and not report['cleanup_errors']
        report['duration_s'] = round(time.monotonic() - run.started, 3)
        path = run.directory / 'character-report.json'
        path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
        print(json.dumps({'REPORT': str(path), 'passed': report['passed'], 'cases': len(report['cases']),
                          'error': str(report.get('error', ''))[:1000]}), flush=True)
        for node in run.sessions:
            node.log.close()
            node.stderr.close()
        run.events.close()
    return 0 if report['passed'] else 1


def main():
    """Select the actual CLI executable and optional copied legacy-test profile."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--legacy-profile', type=Path)
    parser.add_argument('--focus', choices=['all', 'bindings', 'lifecycle', 'legacy'], default='all')
    arguments = parser.parse_args()
    return asyncio.run(exercise(arguments.binary, arguments.legacy_profile, arguments.focus))


if __name__ == '__main__':
    raise SystemExit(main())
