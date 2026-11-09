import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'build_scripts'))
import build_flutter_ohos as build


class OhosPluginStagingTests(unittest.TestCase):
    """Checks native plugin staging preserves files required by video playback."""

    # Exercises native source resolution and package links after staging a plugin.
    def test_native_module_retains_sibling_sources_and_embedding_override(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'video_backend'
            files = {
                'pubspec.yaml': 'name: video_backend\nversion: 1.0.0\n',
                'lib/src/callbacks.cpp': '// Native callbacks\n',
                'cmake/deps.cmake': '# Native SDK dependencies\n',
                'ohos/src/main/cpp/CMakeLists.txt': (
                    'include(../../../../cmake/deps.cmake)\n'
                    'add_library(video_backend ../../../../lib/src/callbacks.cpp)\n'
                ),
                'ohos/hvigorfile.ts': "import { harTasks } from '@ohos/hvigor-ohos-plugin';\n",
                'ohos/oh-package.json5': json.dumps({
                    'name': 'video_backend',
                    'dependencies': {'@ohos/flutter_ohos': 'file:./har/flutter.har'},
                }),
            }
            for relative, content in files.items():
                path = source / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding='utf-8')
            project = root / 'app/ohos'
            entry = project / 'entry/oh-package.json5'
            entry.parent.mkdir(parents=True)
            root_package = project / 'oh-package.json5'
            for manifest, prefix in ((root_package, './'), (entry, '../')):
                manifest.write_text(json.dumps({'dependencies': {
                    'video_player_ohos': f'file:{prefix}.flutter_ohos_plugins/video_player_ohos',
                }}), encoding='utf-8')
            graph = root / 'app/.flutter-plugins-dependencies'
            graph.write_text(json.dumps({'plugins': {'ohos': [
                {'name': 'video_backend', 'path': str(source), 'native_build': True},
                {'name': 'dart_only', 'path': str(root / 'absent'), 'native_build': False},
            ]}}), encoding='utf-8')
            with (
                patch.object(build, 'FLUTTER_PLUGINS_DEPENDENCIES', graph),
                patch.object(build, 'OHOS_STAGED_PLUGINS_DIR', project / '.flutter_ohos_plugins'),
                patch.object(build, 'OHOS_ROOT_PACKAGE', root_package),
                patch.object(build, 'OHOS_ENTRY_PACKAGE', entry),
            ):
                names = build.stage_ohos_flutter_plugins()
                build.write_ohos_package_dependencies(names)
            self.assertEqual(names, ['video_backend'])
            config = json.loads(root_package.read_text(encoding='utf-8'))
            module = project / config['dependencies']['video_backend'].removeprefix('file:')
            cpp = module / 'src/main/cpp'
            self.assertEqual((cpp / '../../../../cmake/deps.cmake').read_text(), files['cmake/deps.cmake'])
            self.assertEqual((cpp / '../../../../lib/src/callbacks.cpp').read_text(), files['lib/src/callbacks.cpp'])
            self.assertEqual((module.parent / 'pubspec.yaml').read_text(), files['pubspec.yaml'])
            staged_package = json.loads((module / 'oh-package.json5').read_text(encoding='utf-8'))
            self.assertEqual(staged_package['dependencies']['@ohos/flutter_ohos'], '')
            self.assertNotIn('video_player_ohos', config['dependencies'])
            self.assertEqual(config['overrides']['video_backend'], config['dependencies']['video_backend'])
            entry_config = json.loads(entry.read_text(encoding='utf-8'))
            self.assertNotIn('video_player_ohos', entry_config['dependencies'])
            entry_module = entry.parent / entry_config['dependencies']['video_backend'].removeprefix('file:')
            self.assertEqual(entry_module.resolve(), module.resolve())


if __name__ == '__main__':
    unittest.main()
