#!/usr/bin/env python3
"""Run real Python code; preserve exit status, diagnostics and root provenance."""
import argparse
import json
from pathlib import Path
from probe_support import execute


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--external-network', action='store_true', help='also require live DNS and certificate-verified HTTPS')
    args = parser.parse_args()
    root = args.root.resolve()
    marker = root / '.operit-arm64-root.json'
    if not marker.is_file() or json.loads(marker.read_text()).get('architecture') != 'aarch64':
        parser.error('--root must be a verified build-owned AArch64 root')
    report = {'root_provenance': json.loads(marker.read_text()), 'guest': []}
    fixtures = Path(__file__).resolve().parent / 'python'
    scripts = [('environment.py', 'python-environment'), ('virtual_filesystem.py', 'python-virtual-filesystem'), ('network.py', 'python-network')]
    if args.external_network:
        scripts.append(('network_external.py', 'python-external-network'))
    for accelerated in (False, True):
        command = [str(args.runtime.resolve()), '--root', str(root), '--stats', '--max-steps', '1000000000',
                   '--bind-ro', str(fixtures) + ':/operit-probes']
        if accelerated:
            command.append('--fixed-acceleration')
        for script, expected in scripts:
            result = execute([*command, '--', '/usr/bin/python3', '/operit-probes/' + script], 30)
            valid = False
            for line in result['stdout'].splitlines():
                try:
                    record = json.loads(line)
                except ValueError:
                    continue
                valid |= record.get('test') == expected and record.get('ok') is True
            result.update(script=script, accelerated=accelerated, passed=result['returncode'] == 0 and valid and not result['timed_out'])
            report['guest'].append(result)
            print(json.dumps({key: result[key] for key in ('script', 'accelerated', 'passed', 'returncode', 'stats')}), flush=True)
    report['passed'] = all(result['passed'] for result in report['guest'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
