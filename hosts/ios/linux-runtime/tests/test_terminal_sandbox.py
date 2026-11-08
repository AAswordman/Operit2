"""Exercise terminal fallback with host PTY access denied by the Apple sandbox."""
import argparse
from pathlib import Path
import subprocess
import tempfile


def literal(path):
    return str(path).replace('\\', '\\\\').replace('"', '\\"')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture', required=True, type=Path)
    parser.add_argument('--terminal', type=Path)
    parser.add_argument('--syscalls', type=Path)
    parser.add_argument('--root', type=Path)
    options = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='operit-terminal-sandbox-') as temporary:
        profile = Path(temporary) / 'deny-pty.sb'
        rules = ['(version 1)', '(allow default)',
                 '(deny file-read* file-write* (literal "/dev/ptmx"))',
                 '(deny process-fork)']
        if options.root:
            # Linux executables must run through the emulator, without host JIT
            # privileges or executable mappings of the downloaded rootfs.
            rules.append(f'(deny file-map-executable (subpath "{literal(options.root.resolve())}"))')
        profile.write_text('\n'.join(rules) + '\n')
        commands = [[str(options.fixture.resolve()), '--require-software']]
        if options.syscalls:
            commands.append([str(options.syscalls.resolve()), '--require-software'])
        if options.terminal and options.root:
            commands.append([str(options.terminal.resolve()), str(options.root.resolve())])
        for command in commands:
            result = subprocess.run(['/usr/bin/sandbox-exec', '-f', str(profile), *command],
                                    capture_output=True, text=True, timeout=45)
            if result.returncode:
                raise AssertionError(f'{Path(command[0]).name}:\n{result.stdout}{result.stderr}')
            print(result.stdout.strip())


if __name__ == '__main__':
    main()
