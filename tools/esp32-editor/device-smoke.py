#!/usr/bin/env python3
"""Opt-in REAL hardware Link smoke test, not an AI-chat end-to-end test.

Requires an already paired/admitted device and firmware with device:health.
Runs the separate Core CLI as a client; never flashes, resets, changes Wi-Fi,
approves requests, grants storage permission, or persists logs/screens/codes.
Each process releases its UART before USB diagnostics use the same port.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

# Existing USB sideband helper; the dash-named file remains independently runnable.
import importlib.util
_spec = importlib.util.spec_from_file_location('device_debug', Path(__file__).with_name('device-debug.py'))
_debug = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_debug)
debug_device = _debug.debug_device


def cli_json(cli, arguments, timeout):
    result = subprocess.run([str(cli), 'cli', '--json', 'link', *arguments],
                            capture_output=True, encoding='utf-8', errors='replace',
                            timeout=timeout, cwd=Path(__file__).resolve().parents[2])
    lines = [line for line in result.stdout.splitlines() if line.strip()]
    value = json.loads(lines[-1]) if lines else None
    if result.returncode != 0 or value is None or isinstance(value, dict) and value.get('error'):
        raise RuntimeError(f'Core CLI failed: {value or result.stderr.strip() or result.returncode}')
    return value


def check_health(previous, current):
    for key in ('uptimeMs', 'heapFreeBytes', 'heapMinimumFreeBytes',
                'largest8bitBlockBytes', 'currentTaskStackHighWaterMark'):
        if not isinstance(current.get(key), (int, float)) or current[key] < 0:
            raise RuntimeError(f'Missing/invalid real hardware metric: {key}')
    if previous and current['uptimeMs'] < previous['uptimeMs']:
        raise RuntimeError('Hardware uptime decreased: possible reboot during test')
    if current['heapFreeBytes'] <= 0 or current['largest8bitBlockBytes'] <= 0:
        raise RuntimeError('Hardware has no usable heap remaining')
    if previous and current['heapMinimumFreeBytes'] > previous['heapMinimumFreeBytes']:
        raise RuntimeError('Heap low-water mark increased: possible reboot during test')


def run_smoke(port, node, cli, count=20, interval=0.2, timeout=20):
    peers = cli_json(cli, ['peers'], timeout)
    if not any(peer.get('nodeId') == node and peer.get('outbound') for peer in peers):
        raise RuntimeError('Explicit target has no outbound pairing; pair it first')
    space = cli_json(cli, ['space', 'show'], timeout)
    if node not in space['members']:
        raise RuntimeError('Target has not joined this device Space; complete real approval first')
    def hardware_health():
        response = debug_device(port, 'health', timeout=min(timeout, 60))
        if response['source'] != 'device-uart':
            raise RuntimeError('Not a physical UART response')
        return response['result']
    baseline = hardware_health()
    check_health(None, baseline)
    previous = baseline
    minimum = maximum = baseline['heapFreeBytes']
    board_id = None
    started = time.monotonic()
    warmed_heap = None
    for index in range(count):
        status = cli_json(cli, ['edge-plugin', node, 'invoke', 'device.status', 'read'], timeout)
        if status.get('boardId') != 'ESP32-2432S028':
            raise RuntimeError(f'Wrong physical board: {status.get("boardId")}')
        if board_id is not None and status['boardId'] != board_id:
            raise RuntimeError('Board identity changed during test')
        board_id = status['boardId']
        # A second authenticated request checks the health plugin and reconnect.
        remote_health = cli_json(cli, ['edge-plugin', node, 'invoke', 'device.status', 'health'], timeout)
        check_health(previous, remote_health)
        current = hardware_health()
        check_health(remote_health, current)
        if index == min(4, count - 1): warmed_heap = current['heapFreeBytes']
        minimum = min(minimum, current['heapFreeBytes'])
        maximum = max(maximum, current['heapFreeBytes'])
        print(json.dumps({'iteration': index + 1, 'status': 'ok',
                          'heapFreeBytes': current['heapFreeBytes'],
                          'uptimeMs': current['uptimeMs']}, ensure_ascii=False), flush=True)
        previous = current
        if interval: time.sleep(interval)
    retained_loss = max(0, warmed_heap - previous['heapFreeBytes'])
    # Ignore initial lazy allocations; continuing growth after warm-up is not
    # a pass merely because all requests happened to return successfully.
    if retained_loss > 2048:
        raise RuntimeError(f'Heap continued shrinking after warm-up: {retained_loss} bytes')
    return {'ok': True, 'source': 'device-uart', 'boardId': board_id, 'nodeId': node,
            'iterations': count, 'authenticatedCalls': count * 2,
            'elapsedSeconds': round(time.monotonic() - started, 2),
            'baseline': baseline, 'final': previous,
            'sampledHeapFreeRangeBytes': [minimum, maximum],
            'postWarmupRetainedLossBytes': retained_loss,
            'heapFreeDeltaBytes': previous['heapFreeBytes'] - baseline['heapFreeBytes'],
            'chatEndToEndTested': False,
            'scope': 'Paired/admitted encrypted Link request-response and serial reconnect; not AI chat'}


def run_live_smoke(bridge, node, count=100, interval=0.2, timeout=30):
    from urllib.request import Request, urlopen
    from urllib.error import HTTPError
    def request(path, body):
        try:
            with urlopen(Request(bridge.rstrip('/') + path, data=json.dumps(body).encode('utf-8'),
                headers={'Content-Type': 'application/json'}), timeout=timeout) as response:
                return json.load(response)['result']
        except HTTPError as error: raise RuntimeError(json.load(error).get('error', str(error))) from error
    peers = request('/command', {'args': ['peers']})
    if not any(p['nodeId'] == node and p['outbound'] for p in peers): raise RuntimeError('Target is not outbound paired')
    space = request('/command', {'args': ['space', 'show']})
    if node not in space['members']: raise RuntimeError('Target is not admitted')
    baseline = request('/debug', {'operation': 'health'})
    previous = baseline
    warmed = None
    started = time.monotonic()
    for index in range(count):
        status = request('/debug', {'operation': 'status'})
        if status.get('boardId') != 'ESP32-2432S028': raise RuntimeError('Not the actual board')
        screen = request('/debug', {'operation': 'screen'})
        if screen.get('renderer') != 'mini' or screen.get('error'): raise RuntimeError(f'Invalid real UI: {screen}')
        current = request('/debug', {'operation': 'health'})
        check_health(previous, current)
        if index == min(4, count-1): warmed = current
        previous = current
        print(json.dumps({'iteration': index+1, 'ok': True, 'page': screen['page'],
            'heapFreeBytes': current['heapFreeBytes'], 'uptimeMs': current['uptimeMs']}, ensure_ascii=False), flush=True)
        if interval: time.sleep(interval)
    loss = max(0,warmed['heapFreeBytes']-previous['heapFreeBytes'])
    if loss > 2048: raise RuntimeError(f'Continuing post-warmup heap loss: {loss}')
    if previous['nvs']['usedEntries'] > warmed['nvs']['usedEntries']:
        raise RuntimeError('Read-only runtime/UI checks accumulated NVS entries')
    return {'ok': True, 'source': 'device-link-serial', 'nodeId': node, 'iterations':count,
        'authenticatedCalls': count*3, 'elapsedSeconds':round(time.monotonic()-started,2),
        'baseline':baseline, 'final':previous, 'postWarmupRetainedLossBytes':loss,
        'chatEndToEndTested':False, 'scope':'Persistent encrypted carrier + actual UI/main-thread dispatch + memory/NVS; AI chat is tested separately'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port')
    parser.add_argument('--bridge')
    parser.add_argument('--node', required=True, help='Explicit already paired real node ID')
    default_cli = Path(__file__).resolve().parents[2] / 'apps/cli/target/release' / (
        'operit2.exe' if sys.platform == 'win32' else 'operit2')
    parser.add_argument('--cli', type=Path, default=default_cli)
    parser.add_argument('--count', type=int, default=20)
    parser.add_argument('--interval', type=float, default=0.2)
    parser.add_argument('--timeout', type=float, default=20)
    args = parser.parse_args()
    if not 1 <= args.count <= 1000 or not 0 <= args.interval <= 60 or not 0 < args.timeout <= 60:
        parser.error('count: 1..1000, interval: 0..60, timeout: (0,60]')
    if bool(args.port) == bool(args.bridge): parser.error('Use exactly one of --port or --bridge')
    if not args.bridge and not args.cli.is_file():
        parser.error('Build the separate Core CLI or specify --cli')
    try:
        result = run_live_smoke(args.bridge, args.node, args.count, args.interval, args.timeout) if args.bridge else run_smoke(args.port, args.node, args.cli, args.count, args.interval, args.timeout)
        print(json.dumps(result, ensure_ascii=False, indent=2))
    except (Exception, KeyboardInterrupt) as error:
        print(json.dumps({'ok': False, 'error': str(error), 'chatEndToEndTested': False},
                         ensure_ascii=False), file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
