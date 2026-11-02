#!/usr/bin/env python3
"""Run actual Node startup, worker_threads and libuv probes; failures remain failures."""
import argparse
import json
import os
from pathlib import Path
import platform
from probe_support import execute

FIXTURES = Path(__file__).resolve().parent / 'node'
TESTS = ('version', 'basic', 'workers', 'threadpool', 'network', 'watch', 'network-external')


def validated(result, test):
    if result['returncode'] != 0 or result['timed_out']:
        return False
    if test == 'version':
        return result['stdout'].strip().startswith('v24.')
    for line in result['stdout'].splitlines():
        try:
            record = json.loads(line)
        except ValueError:
            continue
        if record.get('test') != test or record.get('ok') is not True:
            continue
        if test == 'workers':
            return record.get('workers') == 4 and record.get('increments') == 40000
        if test == 'threadpool':
            return record.get('jobs') == 8 and record.get('completed') == 8
        if test == 'network-external':
            return record.get('tls') in ('TLSv1.2', 'TLSv1.3') and 200 <= record.get('status', 0) < 400
        if test == 'watch':
            return record.get('events', 0) > 0 and record.get('closed') is True
        if test == 'network':
            return record.get('tcp') is True and record.get('udp') is True
        return test == 'basic' and record.get('arch') == 'arm64'
    return False


def save_report(report, output, complete=False):
    report['complete'] = complete
    report['passed'] = complete and all(result['passed'] for result in [*report['guest'], *report['host_control']])
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_suffix(output.suffix + '.tmp')
    temporary.write_text(json.dumps(report, indent=2) + '\n')
    temporary.replace(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--root', type=Path, required=True, help='build-owned, verified root containing /usr/bin/node')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--external-network', action='store_true', help='also require live DNS and certificate-verified HTTPS')
    parser.add_argument('--host-node', type=Path, help='optional separate host control for worker/pool fixtures')
    parser.add_argument('--max-steps', type=int, default=200000000)
    parser.add_argument('--timeout', type=float, default=30)
    parser.add_argument('--cpus', type=int, nargs='+', default=[1], help='guest CPU counts to compare, 1..64')
    parser.add_argument('--tests', choices=TESTS, nargs='+', help='run only the selected unchanged fixtures')
    args = parser.parse_args()
    if any(cpus < 1 or cpus > 64 for cpus in args.cpus):
        parser.error('--cpus values must be in 1..64')
    if args.host_node and not os.access(args.host_node.expanduser(), os.X_OK):
        parser.error('--host-node must name an executable file')
    root = args.root.resolve()
    marker = root / '.operit-arm64-root.json'
    if not marker.is_file() or json.loads(marker.read_text()).get('architecture') != 'aarch64':
        parser.error('--root must be a verified build-owned AArch64 root')
    if not (root / 'usr/bin/node').is_file():
        parser.error('root does not contain /usr/bin/node')
    report = {'host': platform.platform(), 'root_provenance': json.loads(marker.read_text()),
              'node_mode': '--jitless for JS probes; no guest JIT generated', 'guest': [], 'host_control': []}
    tests = list(dict.fromkeys(args.tests or TESTS[:-1]))
    if args.external_network and 'network-external' not in tests:
        tests.append('network-external')
    save_report(report, args.output)
    for cpus in dict.fromkeys(args.cpus):
        for accelerated in (False, True):
            prefix = [str(args.runtime.resolve()), '--root', str(root), '--stats', '--max-steps', str(args.max_steps),
                      '--cpus', str(cpus), '--bind-ro', str(FIXTURES) + ':/operit-probes']
            if accelerated:
                prefix.append('--fixed-acceleration')
            for test in tests:
                arguments = ['--version'] if test == 'version' else ['--jitless', '/operit-probes/' + test.replace('-', '_') + '.cjs']
                result = execute([*prefix, '--', '/usr/bin/node', *arguments], args.timeout)
                result.update(test=test, cpus=cpus, accelerated=accelerated,
                              passed=validated(result, test), node_arguments=arguments)
                report['guest'].append(result)
                save_report(report, args.output)
                print(json.dumps({'scope': 'guest', 'test': test, 'cpus': cpus, 'accelerated': accelerated,
                                  'passed': result['passed'], 'returncode': result['returncode'],
                                  'wall_seconds': result['wall_seconds'],
                                  'host_cpu_seconds': result['host_cpu_seconds'],
                                  'average_host_cores': result['average_host_cores'],
                                  'stats': result['stats']}), flush=True)
    if args.host_node:
        host_node = str(args.host_node.expanduser().absolute())
        report['host_node_version'] = execute([host_node, '--version'], args.timeout)['stdout'].strip()
        for test in ('workers', 'threadpool', 'network', 'watch'):
            if test not in tests:
                continue
            result = execute([host_node, '--jitless', str(FIXTURES / (test + '.cjs'))], args.timeout)
            result.update(test=test, passed=validated(result, test))
            report['host_control'].append(result)
            save_report(report, args.output)
    save_report(report, args.output, complete=True)
    print('report:', args.output.resolve())
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
