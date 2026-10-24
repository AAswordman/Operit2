"""Exercise real preference APIs in two CLI processes connected by authenticated TCP."""
import argparse
import asyncio
import copy
import json
import sys
import time
import types
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / 'apps/cli/tests'))
from space_stress_session import Run

SCHEMAS = {'theme': 'application.themeConfigManager', 'provider': 'preferences.modelConfigManager',
           'tts': 'preferences.ttsConfigManager', 'stt': 'preferences.sttConfigManager'}
LIST = {'theme': 'list', 'provider': 'getProviderProfiles', 'tts': 'getAllTtsConfigs', 'stt': 'getAllSttConfigs'}
STORAGE = 'preferences.preferenceStorageManager'
FILES = ['user_preferences.preferences.json', 'tool_permissions.preferences.json',
         'workspaces.preferences.json', 'skill_visibility.preferences.json',
         'package_manager.preferences.json', 'toolpkg_installation.preferences.json',
         'display_preferences.preferences.json', 'ui_preferences.preferences.json',
         'waifu_settings.preferences.json', 'wake_word_preferences.preferences.json',
         'custom_emoji_settings.preferences.json', 'android_permission_preferences.preferences.json',
         'database_backup_settings.preferences.json', 'persona_card_chat_history.preferences.json',
         'sidebar_dock.preferences.json', 'onboarding_preferences', 'remote_announcement_preferences']


async def call(node, schema, method, **arguments):
    """Call the existing business API through the real CLI's routed local client."""
    return await node.command('call', schema, method, json.dumps(arguments, ensure_ascii=False), timeout=25)


async def records(node, domain):
    """Read the complete domain catalog using its public business API."""
    return await call(node, SCHEMAS[domain], LIST[domain])


def normalized(items):
    """Compare materialized business records independently of list iteration order."""
    return sorted(items, key=lambda item: item['id'])


async def converge(a, b, domain, timeout=12):
    """Wait for two actual catalogs to match and preserve the final observed states."""
    deadline = time.monotonic() + timeout
    while True:
        left, right = await asyncio.gather(records(a, domain), records(b, domain))
        if normalized(left) == normalized(right):
            return left, right
        if time.monotonic() >= deadline:
            raise AssertionError(json.dumps({'domain': domain, 'A': left, 'B': right}, ensure_ascii=False))
        await asyncio.sleep(.3)


async def create(node, domain, name, variation=0):
    """Create valid configurations through their owners, never through filesystem injection."""
    if domain == 'theme':
        snapshot = json.loads((REPO / 'core/crates/runtime/application/src/data/preferences/theme_snapshot_fixture.json').read_text())
        snapshot['fontScale'] = 1.0 + variation / 10
        return await call(node, SCHEMAS[domain], 'create', name=name, snapshot=snapshot)
    endpoint = 'https://sync-test.invalid/v' + str(variation)
    if domain == 'provider':
        identity = await call(node, SCHEMAS[domain], 'createProvider', name=name, providerTypeId='OPENAI', endpoint=endpoint)
        await call(node, SCHEMAS[domain], 'createProviderModel', providerId=identity, modelId='sync-model')
        return await call(node, SCHEMAS[domain], 'getProviderProfile', providerId=identity)
    config = dict(id='', name=name, providerType='OPENAI_COMPATIBLE', endpoint=endpoint,
                  apiKey='synthetic-sync-key', model='sync-model', createdAt=0, updatedAt=0)
    if domain == 'tts':
        config.update(voice='voice-' + str(variation), responseFormat='mp3', speed=1.0)
        method = 'createTtsConfig'
    else:
        config.update(fileFieldName='file', modelFieldName='model', languageFieldName='language',
                      responseTextJsonPath='text', headers=[])
        method = 'createSttConfig'
    return await call(node, SCHEMAS[domain], method, config=config)


async def update(node, domain, record, variation):
    """Update the same retained UUID through the owning business module."""
    changed = copy.deepcopy(record)
    if domain == 'theme':
        changed['snapshot']['fontScale'] = variation
        return await call(node, SCHEMAS[domain], 'update', id=changed['id'], name=changed['name'], snapshot=changed['snapshot'])
    changed['endpoint'] = 'https://sync-test.invalid/update-' + str(variation)
    if domain == 'provider':
        return await call(node, SCHEMAS[domain], 'updateProviderProfile', provider=changed)
    return await call(node, SCHEMAS[domain], 'updateTtsConfig' if domain == 'tts' else 'updateSttConfig', config=changed)


async def select(node, domain, identity):
    """Select one configuration through its business-owned reference API."""
    if domain == 'provider':
        return await call(node, 'preferences.functionalConfigManager', 'setModelForFunction', functionType='CHAT', providerId=identity, modelId='sync-model')
    return await call(node, SCHEMAS[domain], {'theme': 'apply', 'tts': 'setCurrentTtsConfigId', 'stt': 'setCurrentSttConfigId'}[domain], id=identity)


async def selection(node, domain):
    """Read the selected UUID and verify that it resolves through the domain API."""
    if domain == 'provider':
        return (await call(node, 'preferences.functionalConfigManager', 'getModelBindingForFunction', functionType='CHAT'))['providerId']
    if domain == 'theme':
        return (await call(node, SCHEMAS[domain], 'getActive'))['id']
    return await call(node, SCHEMAS[domain], 'getCurrentTtsConfigId' if domain == 'tts' else 'getCurrentSttConfigId')


async def remove(node, domain, identity):
    """Delete a retained configuration using its normal public API."""
    method = {'theme': 'delete', 'provider': 'deleteProvider', 'tts': 'deleteTtsConfig', 'stt': 'deleteSttConfig'}[domain]
    return await call(node, SCHEMAS[domain], method, **{'providerId' if domain == 'provider' else 'id': identity})


async def exercise(binary):
    """Run a reproducible join, conflict, edit, deletion, restart and rejoin matrix."""
    run = Run(types.SimpleNamespace(binary=binary, transport='tcp', focus='all', settle_seconds=15, operations=1))
    report = {'binary': str(binary.resolve()), 'directory': str(run.directory), 'results': [], 'catalogs': {}}
    state = {}

    def result(name, passed, **detail):
        """Persist every observed case rather than hiding later failures after the first error."""
        item = dict(case=name, passed=passed, **detail)
        report['results'].append(item)
        run.event('PASS' if passed else 'FAIL', **item)
        (run.directory / 'preference-report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')

    async def check(name, action):
        """Record a scenario's actual API result without stopping independent remaining scenarios."""
        try:
            detail = await action()
            result(name, True, observed=detail)
        except Exception as error:
            result(name, False, error=str(error))

    try:
        a, b = await run.node('A'), await run.node('B')
        for domain in SCHEMAS:
            left, right = await asyncio.gather(records(a, domain), records(b, domain))
            report['catalogs'][domain] = {'before_A': left, 'before_B': right}
            state[domain] = {}
            for node in [a, b]:
                state[domain][node.name] = {}
                for label, variation in [('Equal', 0), ('Collision', 1 if node is a else 2), ('Unique-' + node.name, 3)]:
                    state[domain][node.name][label] = await create(node, domain, label + '-' + domain, variation)
            await select(a, domain, state[domain]['A']['Equal']['id'])
            await select(b, domain, state[domain]['B']['Equal']['id'])
        await call(a, STORAGE, 'setPreferences', fileName='sync_matrix.preferences.json', values={'only_A': True, 'collision': 'A', 'nested': {'A': 1}, 'array': [1, 2], 'delete_me': True})
        await call(b, STORAGE, 'setPreferences', fileName='sync_matrix.preferences.json', values={'only_B': True, 'collision': 'B', 'nested': {'B': 2}, 'array': [3], 'null_value': None})
        await call(a, 'preferences.envPreferences', 'setEnv', key='OPERIT_SYNC_TEST_LOCAL_ONLY', value='node-A-only')
        await call(a, 'preferences.gitHubAuthPreferences', 'saveAuthInfo', accessToken='synthetic-sync-token-not-a-secret', tokenType='bearer', userInfo=None, grantedScope=None)
        await run.pair(b, a)
        await run.join(b, a, a, [a, b])
        result('two-real-cli-pair-and-space-join', True, pids=[a.process.pid, b.process.pid])
        for domain in SCHEMAS:
            async def initial(domain=domain):
                """Verify all pre-join entities and collision policies on both real nodes."""
                left, right = await converge(a, b, domain)
                report['catalogs'][domain].update(after_join_A=left, after_join_B=right)
                equal = [x for x in left if x['name'].startswith('Equal-' + domain)]
                conflict = [x for x in left if x['name'].startswith('Collision-' + domain)]
                unique = [x for x in left if x['name'].startswith('Unique-')]
                assert len(equal) == 1, {'equal_count': len(equal), 'records': left}
                assert len(conflict) == 2 and len({x['name'] for x in conflict}) == 2, conflict
                assert len(unique) == 2, unique
                return {'equal_UUIDs': [x['id'] for x in equal], 'conflict_names': [x['name'] for x in conflict], 'count': len(left)}
            await check(domain + ':prejoin-same-content-dedup-and-different-content-rename', initial)
            async def refs(domain=domain):
                """Require equal selections to retain a live canonical entity on both nodes."""
                left = await records(a, domain)
                aid, bid = await asyncio.gather(selection(a, domain), selection(b, domain))
                assert aid == bid and any(x['id'] == aid and x['name'] == 'Equal-' + domain for x in left), (aid, bid, left)
                return {'A': aid, 'B': bid}
            await check(domain + ':selection-reference-remap', refs)
        # Every declared generic preference file uses the same public owner interface.
        for filename in FILES:
            async def file_case(filename=filename):
                """Confirm independent keys merge in both directions in one declared preference file."""
                key = 'sync_probe_A'
                await call(a, STORAGE, 'setPreference', fileName=filename, key=key, value={'file': filename, 'from': 'A'})
                await b.wait(('call', STORAGE, 'getPreference', json.dumps({'fileName': filename, 'key': key})), lambda x: x == {'file': filename, 'from': 'A'}, timeout=8)
                await call(b, STORAGE, 'setPreference', fileName=filename, key='sync_probe_B', value=['B', 2, None])
                await a.wait(('call', STORAGE, 'getPreference', json.dumps({'fileName': filename, 'key': 'sync_probe_B'})), lambda x: x == ['B', 2, None], timeout=8)
                return 'A鈫払 and B鈫扐 values match'
            await check('file:' + filename, file_case)
        async def scalar_case():
            """Verify distinct keys, conflicting complete values, JSON null and deletion."""
            keys=['only_A','only_B','collision','nested','array','null_value','delete_me']
            left, right = await asyncio.gather(call(a, STORAGE, 'getPreferences', fileName='sync_matrix.preferences.json', keys=keys), call(b, STORAGE, 'getPreferences', fileName='sync_matrix.preferences.json', keys=keys))
            assert left == right and left['only_A'] and left['only_B'] and left['null_value'] is None, (left,right)
            assert left['nested'] in [{'A':1},{'B':2}] and left['array'] in [[1,2],[3]],left
            await call(b, STORAGE, 'removePreference', fileName='sync_matrix.preferences.json', key='delete_me')
            await a.wait(('call', STORAGE, 'getPreference', json.dumps({'fileName':'sync_matrix.preferences.json','key':'delete_me'})),lambda x:x is None,timeout=8)
            return left
        await check('generic:distinct-key-conflict-nested-array-null-delete', scalar_case)
        async def local_case():
            """Verify device-local environment and encrypted authentication do not leak to peers."""
            env=await call(b,'preferences.envPreferences','getAllEnv')
            auth=await call(b,'preferences.gitHubAuthPreferences','getCurrentAccessToken')
            assert 'OPERIT_SYNC_TEST_LOCAL_ONLY' not in env and auth is None,(env,auth)
            return {'B_env':env,'B_github_token':auth}
        await check('node-local:environment-and-github-auth-not-replicated',local_case)
        # Edit an ordinary shared UUID while the other real CLI is offline.
        await b.stop(crash=True)
        updated={}
        for domain in SCHEMAS:
            current=next(x for x in await records(a,domain) if x['id']==state[domain]['A']['Unique-A']['id'])
            updated[domain]=await update(a,domain,current,1.1)
        await b.start()
        for domain in SCHEMAS:
            async def edit_case(domain=domain):
                """Require one original UUID and the latest payload after offline catch-up."""
                left,right=await converge(a,b,domain)
                matches=[x for x in left if x['id']==updated[domain]['id']]
                assert matches==[updated[domain]],matches
                return {'id':updated[domain]['id'],'count':len(matches)}
            await check(domain+':same-UUID-offline-update',edit_case)
        # Mutate then delete a content-deduplicated canonical UUID to expose alias lifecycle issues.
        for domain in SCHEMAS:
            async def canonical_edit(domain=domain):
                """Check that edits do not resurrect the UUID eliminated by content deduplication."""
                current=next(x for x in await records(a,domain) if x['name']=='Equal-'+domain)
                state[domain]['canonical']=current['id']
                changed=await update(a,domain,current,1.2)
                left,right=await converge(a,b,domain)
                equal=[x for x in left if x['name'].startswith('Equal-'+domain)]
                assert len(equal)==1 and equal[0]['id']==changed['id'],{'matching_entities':equal}
                return {'id':changed['id'],'count':len(equal)}
            await check(domain+':edit-deduplicated-entity-no-alias-resurrection',canonical_edit)
            async def canonical_delete(domain=domain):
                """Check that deleting a deduplicated entity removes every historical alias."""
                remaining=next(x for x in await records(a,domain) if x['name']=='Unique-B-'+domain)
                await select(a,domain,remaining['id'])
                await asyncio.sleep(.5)
                await remove(a,domain,state[domain]['canonical'])
                await asyncio.sleep(1)
                left,right=await converge(a,b,domain)
                equal=[x for x in left if x['name'].startswith('Equal-'+domain)]
                assert not equal,{'resurrected':equal}
                return {'remaining':0}
            await check(domain+':delete-deduplicated-entity-no-alias-resurrection',canonical_delete)
        async def rejoin_case():
            """Leave and rejoin with unchanged UUIDs, then require unchanged catalog identity sets."""
            before={d:sorted(x['id'] for x in await records(a,d)) for d in SCHEMAS}
            await b.command('space','leave')
            await a.wait(('space','show'),lambda value:value['members']==[a.node],timeout=12)
            await run.join(b,a,a,[a,b])
            observed={}
            for domain in SCHEMAS:
                left,right=await converge(a,b,domain)
                after=sorted(x['id'] for x in left)
                assert after==before[domain],{'domain':domain,'before':before[domain],'after':after}
                observed[domain]=len(after)
            return observed
        await check('exit-and-rejoin:existing-UUIDs-no-duplicate',rejoin_case)
        for domain in SCHEMAS:
            report['catalogs'][domain]['final_A']=await records(a,domain)
            report['catalogs'][domain]['final_B']=await records(b,domain)
    except Exception as error:
        result('runner-or-setup',False,error=str(error))
    finally:
        for node in reversed(run.sessions):
            if node.process is not None and node.process.returncode is None:
                try:
                    report.setdefault('diagnostics',{})[node.name]=await node.command('space','sync',error=None,timeout=10)
                except Exception as error:
                    report.setdefault('diagnostics',{})[node.name]=str(error)
            await node.stop(crash=True)
        report['passed']=sum(x['passed'] for x in report['results'])
        report['failed']=sum(not x['passed'] for x in report['results'])
        (run.directory/'preference-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
        print(json.dumps({'REPORT':str(run.directory/'preference-report.json'),'passed':report['passed'],'failed':report['failed']}),flush=True)
        run.events.close()
    return 1 if report['failed'] else 0


async def exercise_lifecycle(binary):
    """Isolate default records, offline conflicts and rejoin from alias-lifecycle failures."""
    run=Run(types.SimpleNamespace(binary=binary,transport='tcp',focus='all',settle_seconds=15,operations=1))
    report={'binary':str(binary.resolve()),'directory':str(run.directory),'results':[]}
    state={}

    def result(name,passed,**detail):
        """Save each isolated observation and preserve all actual process evidence."""
        item=dict(case=name,passed=passed,**detail)
        report['results'].append(item)
        run.event('PASS' if passed else 'FAIL',**item)
        (run.directory/'lifecycle-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')

    async def check(name,action):
        """Record independent case failures without treating them as successful synchronization."""
        try:
            result(name,True,observed=await action())
        except Exception as error:
            result(name,False,error=str(error))

    async def target_matches(domain,expected):
        """Require the specified UUID and exact payload on both actual CLI nodes."""
        deadline=time.monotonic()+12
        while True:
            left,right=await asyncio.gather(records(a,domain),records(b,domain))
            hits=[[x for x in rows if x['id']==expected['id']] for rows in [left,right]]
            if hits==[[expected],[expected]]:
                return {'id':expected['id'],'copies_A':1,'copies_B':1}
            if time.monotonic()>=deadline:
                raise AssertionError({'expected':expected,'observed':hits})
            await asyncio.sleep(.25)

    try:
        a,b=await run.node('A'),await run.node('B')
        for domain in SCHEMAS:
            left,right=await asyncio.gather(records(a,domain),records(b,domain))
            state[domain]={'defaults_A':left,'defaults_B':right}
            state[domain]['A']=await create(a,domain,'Lifecycle-A-'+domain,1)
            state[domain]['B']=await create(b,domain,'Lifecycle-B-'+domain,2)
        default_provider=state['provider']['defaults_A'][0]
        await update(a,'provider',default_provider,1.3)
        expected_default=await update(b,'provider',state['provider']['defaults_B'][0],1.4)
        await run.pair(b,a)
        await run.join(b,a,a,[a,b])
        await check('default-provider:same-UUID-modified-on-both-before-join',lambda:target_matches('provider',expected_default))
        for domain in SCHEMAS:
            async def default_case(domain=domain):
                """Report strict default-record equality separately from normal catalog entries."""
                left,right=await asyncio.gather(records(a,domain),records(b,domain))
                ids={x['id'] for x in state[domain]['defaults_A']}
                observed=[[x for x in rows if x['id'] in ids] for rows in [left,right]]
                assert normalized(observed[0])==normalized(observed[1]),{'A':observed[0],'B':observed[1]}
                return {'default_ids':sorted(ids),'count':len(observed[0])}
            await check(domain+':default-records-strictly-identical',default_case)
        # Both processes edit the same previously synchronized UUID while disconnected.
        await b.stop(crash=True)
        for domain in SCHEMAS:
            await update(a,domain,state[domain]['A'],1.1)
        await call(a,STORAGE,'setPreferences',fileName='concurrent_matrix.preferences.json',values={'only_A':1,'same':'A','nested':{'A':1},'array':[1,2]})
        await a.stop(crash=True)
        await b.start()
        expected={}
        for domain in SCHEMAS:
            expected[domain]=await update(b,domain,state[domain]['A'],1.2)
        await call(b,STORAGE,'setPreferences',fileName='concurrent_matrix.preferences.json',values={'only_B':2,'same':'B','nested':{'B':2},'array':[3]})
        await a.start()
        for domain in SCHEMAS:
            await check(domain+':offline-concurrent-same-UUID-later-operation-wins',lambda domain=domain:target_matches(domain,expected[domain]))
        async def generic_conflict():
            """Verify whole-root LWW semantics rather than undocumented recursive JSON merging."""
            keys=['only_A','only_B','same','nested','array']
            expected={'only_A':1,'only_B':2,'same':'B','nested':{'B':2},'array':[3]}
            for node in [a,b]:
                await node.wait(('call',STORAGE,'getPreferences',json.dumps({'fileName':'concurrent_matrix.preferences.json','keys':keys})),lambda value:value==expected,timeout=12)
            return expected
        await check('generic:offline-concurrent-per-key-LWW-not-deep-merge',generic_conflict)
        # Deleting a normal UUID must remain deleted over restart and rejoin.
        for domain in SCHEMAS:
            await remove(a,domain,state[domain]['B']['id'])
        await asyncio.sleep(2)
        for domain in SCHEMAS:
            async def delete_case(domain=domain):
                """Check ordinary UUID tombstones independently of canonical alias deletion."""
                for node in [a,b]:
                    rows=await records(node,domain)
                    assert all(x['id']!=state[domain]['B']['id'] for x in rows),rows
                return {'deleted_id':state[domain]['B']['id']}
            await check(domain+':normal-UUID-delete',delete_case)
        async def rejoin():
            """Preserve existing identity sets when a real member leaves and rejoins the Space."""
            before={domain:sorted(x['id'] for x in await records(a,domain)) for domain in SCHEMAS}
            await b.command('space','leave')
            await a.wait(('space','show'),lambda value:set(value['members'])=={a.node},timeout=15)
            await run.join(b,a,a,[a,b])
            await asyncio.sleep(2)
            observed={}
            for domain in SCHEMAS:
                for node in [a,b]:
                    after=sorted(x['id'] for x in await records(node,domain))
                    assert after==before[domain],{'node':node.name,'domain':domain,'before':before[domain],'after':after}
                observed[domain]=before[domain]
            return observed
        await check('exit-and-rejoin:unchanged-UUIDs-and-tombstones',rejoin)
        await b.stop(crash=True)
        await b.start()
        await asyncio.sleep(2)
        for domain in SCHEMAS:
            async def restart_case(domain=domain):
                """Read persisted catalog identity sets after a process restart."""
                rows=await records(b,domain)
                assert any(x['id']==state[domain]['A']['id'] for x in rows),rows
                assert not any(x['id']==state[domain]['B']['id'] for x in rows),rows
                return {'retained_id':state[domain]['A']['id'],'deleted_id':state[domain]['B']['id']}
            await check(domain+':restart-retains-updates-and-deletions',restart_case)
        report['final']={node.name:{domain:await records(node,domain) for domain in SCHEMAS} for node in [a,b]}
    except Exception as error:
        result('runner-or-setup',False,error=str(error))
    finally:
        for node in reversed(run.sessions):
            await node.stop(crash=True)
        report['passed']=sum(x['passed'] for x in report['results'])
        report['failed']=sum(not x['passed'] for x in report['results'])
        (run.directory/'lifecycle-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
        print(json.dumps({'REPORT':str(run.directory/'lifecycle-report.json'),'passed':report['passed'],'failed':report['failed']}),flush=True)
        run.events.close()
    return 1 if report['failed'] else 0


async def exercise_focused(binary):
    """Separate default divergence, canonical alias lifecycles and real API preference setters."""
    run = Run(types.SimpleNamespace(binary=binary, transport='tcp', focus='all', settle_seconds=15, operations=1))
    report = {'binary': str(binary.resolve()), 'directory': str(run.directory), 'results': [], 'snapshots': {}}
    state = {}

    async def check(name, action):
        """Record exact owner API observations without masking them with unrelated catalog differences."""
        try:
            item = dict(case=name, passed=True, observed=await action())
        except Exception as error:
            item = dict(case=name, passed=False, error=str(error))
        report['results'].append(item)
        run.event('PASS' if item['passed'] else 'FAIL', **item)

    try:
        a = await run.node('A')
        await records(a, 'tts')
        await asyncio.sleep(.08)
        b = await run.node('B')
        for domain in SCHEMAS:
            state[domain] = {}
            for node in [a, b]:
                for name, variation in [('Equal', 0), ('Collision', 1 if node is a else 2), ('Different-' + node.name, 3)]:
                    await create(node, domain, name + '-' + domain, variation)
            await create(a, domain, 'Collision-' + domain + '（1）', 0 if domain == 'theme' else 4)
        await run.pair(b, a)
        await run.join(b, a, a, [a, b])
        await asyncio.sleep(1)
        for domain in SCHEMAS:
            left, right = await asyncio.gather(records(a, domain), records(b, domain))
            report['snapshots'][domain] = {'initial_A': left, 'initial_B': right}
            state[domain]['ids'] = {x['id'] for x in left if x['name'].startswith('Equal-' + domain)}
            state[domain]['canonical'] = next(x for x in left if x['name'] == 'Equal-' + domain)
            async def initial(domain=domain, left=left, right=right):
                """Compare custom records and check pre-existing suffix reservation independently of defaults."""
                custom = [[x for x in rows if x['name'].startswith(('Equal-', 'Collision-', 'Different-'))] for rows in [left, right]]
                assert normalized(custom[0]) == normalized(custom[1]), custom
                equal = [x for x in custom[0] if x['name'] == 'Equal-' + domain]
                collision = [x for x in custom[0] if x['name'].startswith('Collision-' + domain)]
                different = [x for x in custom[0] if x['name'].startswith('Different-')]
                assert len(equal) == 1 and len(collision) == 3 and len({x['name'] for x in collision}) == 3 and len(different) == 2, custom
                return {'equal_count': len(equal), 'collision_names': [x['name'] for x in collision], 'different_names_same_content': len(different)}
            await check(domain + ':dedup-reserved-suffix-and-different-name-retention', initial)
        async def default_tts():
            """Compare the exact system default record after deliberately staggered local initialization."""
            rows = [next(x for x in await records(node, 'tts') if x['id'] == 'system_tts_default') for node in [a, b]]
            report['snapshots']['system_tts_default'] = {'A': rows[0], 'B': rows[1]}
            assert rows[0] == rows[1], json.dumps({'A': rows[0], 'B': rows[1]})
            return rows[0]
        await check('tts:staggered-default-initialization-strict-convergence', default_tts)
        for source, target in [(a, b), (b, a)]:
            for command, arguments, expected in [
                ('thinking', ['off' if source is a else 'on'], {'enableThinkingMode': source is b}),
                ('thinking-quality', ['2' if source is a else '3'], {'thinkingQualityLevel': 2 if source is a else 3}),
                ('stream', ['off' if source is a else 'on'], {'streamOutputEnabled': source is b}),
                ('media-history', ['37', '41'] if source is a else ['43', '47'], {'maxImageHistoryUserTurns': 37 if source is a else 43, 'maxMediaHistoryUserTurns': 41 if source is a else 47}),
                ('mcp-timeout', ['7' if source is a else '9'], {'mcpStartupTimeoutSeconds': 7 if source is a else 9}),
            ]:
                async def api_case(source=source, target=target, command=command, arguments=arguments, expected=expected):
                    """Exercise real CLI preference setters and assert exact peer getter values."""
                    await source.command('core', 'prefs', command, *arguments)
                    await target.wait(('core', 'prefs', 'show'), lambda value: all(value.get(k) == v for k, v in expected.items()), timeout=10)
                    return expected
                await check('api:' + command + ':' + source.name + '-to-' + target.name, api_case)
            async def language_case(source=source, target=target):
                """Read application language through its owning manager rather than a transport marker."""
                language = 'en' if source is a else 'zh'
                await call(source, 'preferences.userPreferencesManager', 'saveAppLanguage', languageCode=language)
                await target.wait(('call', 'preferences.userPreferencesManager', 'getCurrentLanguage', '{}'), lambda value: value == language, timeout=10)
                return {'language': language}
            await check('user:language:' + source.name + '-to-' + target.name, language_case)
        for domain in SCHEMAS:
            async def canonical_edit(domain=domain):
                """Check canonical edits on unselected entities so active-theme errors cannot mask aliases."""
                changed = await update(a, domain, state[domain]['canonical'], 1.2)
                deadline = time.monotonic() + 12
                while True:
                    rows = await asyncio.gather(records(a, domain), records(b, domain))
                    hits = [[x for x in items if x['name'].startswith('Equal-' + domain)] for items in rows]
                    if hits == [[changed], [changed]] or time.monotonic() >= deadline:
                        break
                    await asyncio.sleep(.25)
                report['snapshots'][domain]['after_edit'] = {'A': hits[0], 'B': hits[1]}
                assert hits == [[changed], [changed]], json.dumps({'A': hits[0], 'B': hits[1]})
                return {'id': changed['id']}
            await check(domain + ':edit-deduplicated-unselected-no-alias-resurrection', canonical_edit)
            async def canonical_delete(domain=domain):
                """Check canonical deletion separately from the already observed edit result."""
                await remove(a, domain, state[domain]['canonical']['id'])
                deadline = time.monotonic() + 12
                while True:
                    rows = await asyncio.gather(records(a, domain), records(b, domain))
                    hits = [[x for x in items if x['name'].startswith('Equal-' + domain)] for items in rows]
                    if hits == [[], []] or time.monotonic() >= deadline:
                        break
                    await asyncio.sleep(.25)
                report['snapshots'][domain]['after_delete'] = {'A': hits[0], 'B': hits[1]}
                assert hits == [[], []], json.dumps({'A': hits[0], 'B': hits[1]})
                return {'remaining': 0}
            await check(domain + ':delete-deduplicated-unselected-no-alias-resurrection', canonical_delete)
    except Exception as error:
        report['results'].append(dict(case='runner-or-setup', passed=False, error=str(error)))
    finally:
        for node in reversed(run.sessions):
            await node.stop(crash=True)
        report['passed'] = sum(x['passed'] for x in report['results'])
        report['failed'] = sum(not x['passed'] for x in report['results'])
        path = run.directory / 'focused-report.json'
        path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
        print(json.dumps({'REPORT': str(path), 'passed': report['passed'], 'failed': report['failed']}), flush=True)
        run.events.close()
    return 1 if report['failed'] else 0


async def exercise_api_extra(binary):
    """Exercise cumulative token statistics and tool visibility maps through their actual APIs."""
    run = Run(types.SimpleNamespace(binary=binary, transport='tcp', focus='all', settle_seconds=15, operations=1))
    report = {'binary': str(binary.resolve()), 'directory': str(run.directory), 'results': []}
    api = 'preferences.apiPreferences'
    model = 'synthetic-provider/sync-model'

    async def tokens(node, amounts):
        """Add synthetic usage without invoking any external model provider."""
        await call(node, api, 'updateTokensForProviderModel', providerModel=model, inputTokens=amounts[0], outputTokens=amounts[1], cachedInputTokens=amounts[2])

    async def check(name, action):
        """Persist exact semantic API failures independently of subsequent observations."""
        try:
            item = dict(case=name, passed=True, observed=await action())
        except Exception as error:
            item = dict(case=name, passed=False, error=str(error))
        report['results'].append(item)
        run.event('PASS' if item['passed'] else 'FAIL', **item)

    try:
        a, b = await run.node('A'), await run.node('B')
        await run.pair(b, a)
        await run.join(b, a, a, [a, b])
        await tokens(a, [10, 1, 2])
        await b.wait(('call', api, 'getAllProviderModelTokens', '{}'), lambda value: value.get(model) == [10, 1, 2], timeout=12)
        await tokens(b, [20, 2, 4])
        await a.wait(('call', api, 'getAllProviderModelTokens', '{}'), lambda value: value.get(model) == [30, 3, 6], timeout=12)
        await call(a, api, 'saveToolPromptVisibilityMap', visibilityMap={'sync_seed': True})
        await b.wait(('call', api, 'getToolPromptVisibilityMap', '{}'), lambda value: value == {'sync_seed': True}, timeout=12)
        report['results'].append(dict(case='api:sequential-cumulative-token-increments', passed=True, observed=[30, 3, 6]))
        await b.stop(crash=True)
        await tokens(a, [100, 10, 20])
        await call(a, api, 'saveToolPromptVisibility', toolName='sync_tool_A', isVisible=False)
        await a.stop(crash=True)
        await b.start()
        await tokens(b, [200, 20, 40])
        await call(b, api, 'saveToolPromptVisibility', toolName='sync_tool_B', isVisible=False)
        await a.start()
        async def token_case():
            """Require offline usage increments from both devices to survive as cumulative statistics."""
            deadline = time.monotonic() + 12
            while True:
                observed = await asyncio.gather(call(a, api, 'getAllProviderModelTokens'), call(b, api, 'getAllProviderModelTokens'))
                if all(value.get(model) == [330, 33, 66] for value in observed) or time.monotonic() >= deadline:
                    break
                await asyncio.sleep(.25)
            report['tokens'] = {'expected': [330, 33, 66], 'A': observed[0], 'B': observed[1]}
            assert all(value.get(model) == [330, 33, 66] for value in observed), json.dumps(report['tokens'])
            return report['tokens']
        await check('api:offline-concurrent-token-increments-not-lost', token_case)
        async def visibility_case():
            """Require edits to different tools not to erase each other after offline concurrent changes."""
            expected = {'sync_seed': True, 'sync_tool_A': False, 'sync_tool_B': False}
            deadline = time.monotonic() + 12
            while True:
                observed = await asyncio.gather(call(a, api, 'getToolPromptVisibilityMap'), call(b, api, 'getToolPromptVisibilityMap'))
                if observed == [expected, expected] or time.monotonic() >= deadline:
                    break
                await asyncio.sleep(.25)
            report['visibility'] = {'expected': expected, 'A': observed[0], 'B': observed[1]}
            assert observed == [expected, expected], json.dumps(report['visibility'])
            return report['visibility']
        await check('api:offline-concurrent-distinct-tool-visibility-edits-not-lost', visibility_case)
        report['diagnostics'] = {node.name: await node.command('space', 'sync') for node in [a, b]}
    except Exception as error:
        report['results'].append(dict(case='runner-or-setup', passed=False, error=str(error)))
    finally:
        for node in reversed(run.sessions):
            await node.stop(crash=True)
        report['passed'] = sum(x['passed'] for x in report['results'])
        report['failed'] = sum(not x['passed'] for x in report['results'])
        path = run.directory / 'api-extra-report.json'
        path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
        print(json.dumps({'REPORT': str(path), 'passed': report['passed'], 'failed': report['failed']}), flush=True)
        run.events.close()
    return 1 if report['failed'] else 0


async def exercise_history_divergence(binary):
    """Retain independently edited UUIDs even when their historical creation snapshots were identical."""
    run = Run(types.SimpleNamespace(binary=binary, transport='tcp', focus='all', settle_seconds=15, operations=1))
    report = {'binary': str(binary.resolve()), 'directory': str(run.directory), 'results': [], 'before': {}, 'after': {}}
    try:
        a, b = await run.node('A'), await run.node('B')
        expected = {}
        for domain in SCHEMAS:
            left = await create(a, domain, 'History-Equal-' + domain)
            right = await create(b, domain, 'History-Equal-' + domain)
            await update(a, domain, left, 1.1 if domain == 'theme' else 1)
            await update(b, domain, right, 1.2 if domain == 'theme' else 2)
            expected[domain] = {left['id'], right['id']}
            await select(a, domain, left['id'])
            await select(b, domain, right['id'])
            report['before'][domain] = dict(zip(['A', 'B'], await asyncio.gather(records(a, domain), records(b, domain))))
        await run.pair(b, a)
        await run.join(b, a, a, [a, b])
        for domain in SCHEMAS:
            left, right = await converge(a, b, domain)
            selected = await asyncio.gather(selection(a, domain), selection(b, domain))
            retained = [item for item in left if item['id'] in expected[domain]]
            report['after'][domain] = {'A': left, 'B': right, 'selection': selected}
            passed = len(retained) == 2 and len({item['name'] for item in retained}) == 2 and all(identity in expected[domain] for identity in selected)
            if domain == 'theme':
                passed = passed and sorted(item['snapshot']['fontScale'] for item in retained) == [1.1, 1.2]
            else:
                passed = passed and {item['endpoint'] for item in retained} == {'https://sync-test.invalid/update-1', 'https://sync-test.invalid/update-2'}
            result = {'case': domain + ':historically-equal-independently-edited-before-join', 'passed': passed, 'expectedIds': sorted(expected[domain]), 'observedIds': [item['id'] for item in retained]}
            report['results'].append(result)
            run.event('PASS' if passed else 'FAIL', **result)
    except Exception as error:
        report['results'].append({'case': 'runner-or-setup', 'passed': False, 'error': str(error)})
    finally:
        for node in reversed(run.sessions):
            await node.stop(crash=True)
        report['passed'] = sum(item['passed'] for item in report['results'])
        report['failed'] = sum(not item['passed'] for item in report['results'])
        path = run.directory / 'history-divergence-report.json'
        path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
        print(json.dumps({'REPORT': str(path), 'passed': report['passed'], 'failed': report['failed']}), flush=True)
        run.events.close()
    return 1 if report['failed'] else 0


async def exercise_history_paged(binary):
    """Transfer more than one actual transport page before deciding whether current UUIDs are equal."""
    run = Run(types.SimpleNamespace(binary=binary, transport='tcp', focus='all', settle_seconds=15, operations=1))
    report = {'directory': str(run.directory), 'results': [], 'after': {}}
    try:
        a, b = await run.node('A'), await run.node('B')
        pairs = {}
        for domain in SCHEMAS:
            pairs[domain] = (await create(a, domain, 'Paged-History-' + domain), await create(b, domain, 'Paged-History-' + domain))
        for index in range(600):
            await call(a, STORAGE, 'setPreferences', fileName='paged_identity.preferences.json', values={'padding': index})
        for domain, (left, right) in pairs.items():
            await update(a, domain, left, 1.1 if domain == 'theme' else 1)
            await select(a, domain, left['id'])
            await select(b, domain, right['id'])
        await run.pair(b, a)
        await run.join(b, a, a, [a, b])
        for domain, pair in pairs.items():
            expected = {item['id'] for item in pair}
            left, right = await converge(a, b, domain)
            retained = [item for item in left if item['id'] in expected]
            passed = len(retained) == 2 and len({item['name'] for item in retained}) == 2
            passed = passed and (sorted(item['snapshot']['fontScale'] for item in retained) == [1.0, 1.1] if domain == 'theme' else {item['endpoint'] for item in retained} == {'https://sync-test.invalid/v0', 'https://sync-test.invalid/update-1'})
            result = {'case': domain + ':page-boundary-does-not-merge-historical-creation', 'passed': passed, 'expectedIds': sorted(expected), 'observedIds': [item['id'] for item in retained]}
            report['results'].append(result)
            report['after'][domain] = {'A': left, 'B': right}
            run.event('PASS' if passed else 'FAIL', **result)
    except Exception as error:
        report['results'].append({'case': 'runner-or-setup', 'passed': False, 'error': str(error)})
    finally:
        for node in reversed(run.sessions):
            await node.stop(crash=True)
        report['passed'] = sum(item['passed'] for item in report['results'])
        report['failed'] = sum(not item['passed'] for item in report['results'])
        path = run.directory / 'history-paged-report.json'
        path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
        print(json.dumps({'REPORT': str(path), 'passed': report['passed'], 'failed': report['failed']}), flush=True)
        run.events.close()
    return 1 if report['failed'] else 0


async def main():
    """Select the current compiled CLI and run an actual TCP scenario matrix."""
    parser=argparse.ArgumentParser()
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--lifecycle',action='store_true')
    parser.add_argument('--focused',action='store_true')
    parser.add_argument('--api-extra',action='store_true')
    parser.add_argument('--history-divergence',action='store_true')
    parser.add_argument('--history-paged',action='store_true')
    args=parser.parse_args()
    if args.history_paged:
        return await exercise_history_paged(args.binary)
    if args.history_divergence:
        return await exercise_history_divergence(args.binary)
    if args.api_extra:
        return await exercise_api_extra(args.binary)
    if args.focused:
        return await exercise_focused(args.binary)
    if args.lifecycle:
        return await exercise_lifecycle(args.binary)
    return await exercise(args.binary)


if __name__=='__main__':
    raise SystemExit(asyncio.run(main()))
