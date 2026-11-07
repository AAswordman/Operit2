"""Bind an authorized directory when the Apple sandbox denies opening its parent."""
import argparse
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture', required=True, type=Path)
    options = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='operit-bind-sandbox-') as temporary:
        # Sandbox literal filters match canonical paths, including /private/tmp.
        base = Path(temporary).resolve()
        parent = base / 'containers'
        source = parent / 'authorized-container'
        source.mkdir(parents=True)
        (source / 'message').write_text('sandbox-directory-data')
        alias = parent / 'directory-link'
        alias.symlink_to(source, target_is_directory=True)
        root = base / 'guest-root'
        root.mkdir()
        profile = base / 'deny-parent.sb'
        parent_literal = str(parent).replace('\\', '\\\\').replace('"', '\\"')
        profile.write_text('(version 1)\n(allow default)\n'
                           f'(deny file-read-data (literal "{parent_literal}"))\n')
        for path in [str(source), str(source) + '/', str(alias)]:
            result = subprocess.run(
                ['/usr/bin/sandbox-exec', '-f', str(profile), str(options.fixture.resolve()),
                 '--sandbox-bind', str(root), path, str(parent)],
                capture_output=True, text=True, timeout=10)
            if result.returncode:
                raise AssertionError(f'{path}:\n{result.stdout}{result.stderr}')
            print(f'{Path(path).name}: {result.stdout.strip()}')


if __name__ == '__main__':
    main()
