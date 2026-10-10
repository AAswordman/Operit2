import importlib.util
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from io import BytesIO
from urllib.parse import urlsplit, parse_qs

spec = importlib.util.spec_from_file_location('sync_packages', Path(__file__).with_name('sync_plugin_packages.py'))
sync = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = sync
spec.loader.exec_module(sync)


class HotReloadTests(unittest.TestCase):
    """Checks upload acknowledgements without writing to a real runtime."""

    def test_upload_and_failed_commit(self):
        """Records signatures only after a successful remote commit."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'workflow.toolpkg').write_bytes(b'package' * 5000)
            actions = []
            fail = False

            def respond(url, timeout):
                """Emulates VM discovery and the extension response envelope."""
                parsed = urlsplit(url)
                params = parse_qs(parsed.query)
                if parsed.path.endswith('getVM'):
                    result = {'isolates': [{'id': 'isolates/1'}]}
                elif parsed.path.endswith('getIsolate'):
                    result = {'extensionRPCs': ['ext.operit.reloadPlugins']}
                else:
                    action = params['action'][0]
                    actions.append(action)
                    if fail and action == 'commit':
                        return BytesIO(json.dumps({'error': {'message': 'write failed'}}).encode())
                    result = {'ok': True}
                return BytesIO(json.dumps({'result': result}).encode())

            arguments = dict(state_key='test', label='test', dry_run=False,
                             disabled=False, timeout_seconds=5,
                             vm_service='ws://127.0.0.1:1234/auth/ws')
            with patch.object(sync.urllib.request, 'urlopen', side_effect=respond):
                sync._maybe_hot_reload_output(root, root, **arguments)
                self.assertEqual(actions[0], 'begin')
                self.assertGreater(actions.count('chunk'), 1)
                self.assertEqual(actions[-1], 'commit')
                state = root / sync.HOT_RELOAD_STATE_FILE
                self.assertTrue(state.exists())
                state.unlink()
                fail = True
                with self.assertRaises(RuntimeError):
                    sync._maybe_hot_reload_output(root, root, **arguments)
                self.assertFalse(state.exists())

    # Verifies build-time plugin synchronization never requires a running app.
    def test_android_build_sync_does_not_contact_vm_service(self):
        """Checks that Android build arguments sync both plugin sets without a runtime."""
        gradle = (Path(__file__).resolve().parents[2] /
                  'apps/flutter/app/android/app/build.gradle.kts').read_text(encoding='utf-8')
        task = re.search(
            r'^val syncOperitPlugins = tasks\.register<Exec>\("syncOperitPlugins"\) \{(.*?)^\}',
            gradle, re.MULTILINE | re.DOTALL,
        )
        self.assertIsNotNone(task)
        command = re.search(r'commandLine\((.*?)\)', task.group(1), re.DOTALL)
        self.assertIsNotNone(command)
        arguments = re.findall(r'"([^"\n]*)"', command.group(1))
        self.assertEqual(arguments, ['--source', 'runtime', '--no-hot-reload'])

        with (
            patch.object(sys, 'argv', ['sync_plugin_packages.py', *arguments]),
            patch.object(sync, '_generate_plugin_sdk_types'),
            patch.object(sync, '_sync', return_value=(0, 0, 0)) as synchronize,
            patch.object(sync, '_discover_vm_service') as discover,
            patch.object(sync.urllib.request, 'urlopen') as request,
            patch('builtins.print'),
        ):
            self.assertEqual(sync.main(), 0)
            self.assertEqual(
                [call.args[0].name for call in synchronize.call_args_list],
                ['buildin', 'external'],
            )
            discover.assert_not_called()
            request.assert_not_called()

    def test_discovers_flutter_development_service(self):
        """Extracts the authenticated VM service URI from the Flutter service process."""
        command_lines = [
            'dart.exe development-service --vm-service-uri="http://127.0.0.1:4906/token/"'
        ]
        with patch.object(sync, '_running_process_command_lines', return_value=command_lines):
            self.assertEqual(
                sync._discover_vm_service(),
                'http://127.0.0.1:4906/token/',
            )


class ScriptPackedDependencyTests(unittest.TestCase):
    """Checks dependency installation before script-managed package builds."""

    # Creates a script-managed package with an optional dependency declaration.
    def create_package(self, root, *, with_dependencies):
        package = root / 'music_studio'
        package.mkdir()
        data = {'scripts': {'pack:toolpkg': 'node scripts/pack.mjs'}}
        if with_dependencies:
            data['devDependencies'] = {'typescript': '5.9.2'}
        (package / 'package.json').write_text(json.dumps(data), encoding='utf-8')
        plan = sync.SyncPlanItem('copy-script-toolpkg', package, 'music_studio.toolpkg')
        return package, [plan]

    # Verifies installation precedes packaging in both execution and dry-run modes.
    def test_installs_locked_dependencies_before_pack(self):
        for dry_run in (False, True):
            with self.subTest(dry_run=dry_run), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                package, plans = self.create_package(root, with_dependencies=True)
                with (
                    patch.object(sync.shutil, 'which', return_value='corepack'),
                    patch.object(sync, '_run_checked_command') as run,
                ):
                    sync._prebuild_plans(root, root, plans, dry_run=dry_run)
                self.assertEqual([call.args[0] for call in run.call_args_list], [
                    ['corepack', 'pnpm', 'install', '--frozen-lockfile'],
                    ['corepack', 'pnpm', 'run', 'pack:toolpkg'],
                ])
                for call in run.call_args_list:
                    self.assertEqual(call.args[1], package)
                    self.assertEqual(call.kwargs, {'dry_run': dry_run})

    # Verifies a failed dependency install stops the package build immediately.
    def test_install_failure_stops_pack(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, plans = self.create_package(root, with_dependencies=True)
            with (
                patch.object(sync.shutil, 'which', return_value='corepack'),
                patch.object(sync, '_run_checked_command',
                             side_effect=RuntimeError('install failed')) as run,
            ):
                with self.assertRaisesRegex(RuntimeError, 'install failed'):
                    sync._prebuild_plans(root, root, plans, dry_run=False)
            run.assert_called_once()
            self.assertEqual(run.call_args.args[0],
                             ['corepack', 'pnpm', 'install', '--frozen-lockfile'])

    # Verifies packages without dependencies do not require a dependency lockfile.
    def test_package_without_dependencies_runs_pack(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package, plans = self.create_package(root, with_dependencies=False)
            with (
                patch.object(sync.shutil, 'which', return_value='corepack'),
                patch.object(sync, '_run_checked_command') as run,
            ):
                sync._prebuild_plans(root, root, plans, dry_run=False)
            run.assert_called_once_with(
                ['corepack', 'pnpm', 'run', 'pack:toolpkg'], package, dry_run=False)


if __name__ == '__main__':
    unittest.main()
