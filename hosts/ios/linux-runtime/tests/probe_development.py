#!/usr/bin/env python3
"""Exercise installed development tools through real guest processes."""
import argparse
import json
from pathlib import Path
from probe_support import execute

FIXTURES = Path(__file__).resolve().parent / 'development'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--external-network', action='store_true')
    parser.add_argument('--timeout', type=float, default=180)
    parser.add_argument('--max-steps', type=int, default=20_000_000_000)
    parser.add_argument('--no-fixed-acceleration', action='store_true')
    options = parser.parse_args()
    root = options.root.resolve()
    provenance = root / '.operit-arm64-root.json'
    if not provenance.is_file() or json.loads(provenance.read_text()).get('architecture') != 'aarch64':
        parser.error('--root must be a verified AArch64 development root')
    prefix = [str(options.runtime.resolve()), '--root', str(root), '--stats',
              '--max-steps', str(options.max_steps), '--bind-ro', str(FIXTURES) + ':/operit-probes']
    if not options.no_fixed_acceleration:
        prefix.append('--fixed-acceleration')
    checks = [(name, ['/bin/sh', '/operit-probes/' + name + '.sh'], expected) for name, expected in (
        ('compiler', 'GCC_PTHREAD_FORK_OK'), ('git', 'GIT_COMMIT_OK'),
        ('python', 'PYTHON_VENV_PIP_OK'), ('npm', 'NPM_INSTALL_SCRIPT_OK'))]
    if options.external_network:
        checks.append(('curl', ['/usr/bin/curl', '--ipv4', '--fail', '--silent', '--show-error',
                               '--max-time', '20', '--output', '/dev/null',
                               'https://dl-cdn.alpinelinux.org/alpine/v3.23/main/aarch64/APKINDEX.tar.gz'], ''))
    report = {'root_provenance': json.loads(provenance.read_text()), 'checks': []}
    for name, arguments, expected in checks:
        result = execute([*prefix, '--', *arguments], options.timeout)
        result.update(test=name, passed=result['returncode'] == 0 and not result['timed_out']
                      and expected in result['stdout'])
        report['checks'].append(result)
        print(json.dumps({'test': name, 'passed': result['passed'],
                          'returncode': result['returncode'], 'stats': result['stats']}), flush=True)
        options.output.parent.mkdir(parents=True, exist_ok=True)
        report['passed'] = all(check['passed'] for check in report['checks'])
        options.output.write_text(json.dumps(report, indent=2) + '\n')
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
