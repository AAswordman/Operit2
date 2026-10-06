#!/usr/bin/env python3
"""Read/click the actual ESP32 UI over its existing USB UART (not the simulator).
OPD1/OPD2 envelopes: magic + big-endian length + payload + CRC32(payload).
Request payload: uint32 request id + uint8 op + UTF-8 argument. Response: id + JSON.
No reset, no pairing bypass, no Wi-Fi/TCP debugging endpoint.
"""
import argparse
import json
import secrets
import struct
import sys
import time
import zlib

MAX_RESPONSE = 16 * 1024
OPS = {'screen': 1, 'tree': 2, 'tap': 3, 'swipe': 4, 'health': 5, 'draft': 6}


def request_frame(request_id, operation, argument=''):
    if operation not in OPS:
        raise ValueError('Unknown operation')
    if operation in ('screen', 'tree', 'health') and argument:
        raise ValueError('Inspection takes no argument')
    if operation in ('tap', 'swipe') and not argument:
        raise ValueError('tap/swipe requires a target')
    if '\0' in argument:
        raise ValueError('Target must not contain NUL')
    payload = struct.pack('>IB', request_id, OPS[operation]) + argument.encode('utf-8')
    if len(payload) > 128:
        raise ValueError('Target exceeds bounded request capacity')
    return b'OPD1' + struct.pack('>I', len(payload)) + payload + struct.pack('>I', zlib.crc32(payload))


class ResponseReader:
    """Bounded, noise-tolerant scanner; discard unrelated Link/log output."""
    def __init__(self, request_id):
        self.pending = bytearray()
        self.request_id = request_id

    def feed(self, data):
        self.pending.extend(data)
        while True:
            at = self.pending.find(b'OPD2')
            if at < 0:
                del self.pending[:-3]
                return None
            del self.pending[:at]
            if len(self.pending) < 8:
                return None
            length = struct.unpack('>I', self.pending[4:8])[0]
            if not 6 <= length <= MAX_RESPONSE:
                del self.pending[:1]
                continue
            total = length + 12
            if len(self.pending) < total:
                return None
            payload = bytes(self.pending[8:8+length])
            crc = struct.unpack('>I', self.pending[8+length:total])[0]
            if zlib.crc32(payload) != crc:
                del self.pending[:1]
                continue
            del self.pending[:total]
            if struct.unpack('>I', payload[:4])[0] != self.request_id:
                continue
            return json.loads(payload[4:].decode('utf-8'))


def debug_device(port, operation, argument='', timeout=12):
    import serial
    request_id = secrets.randbits(32)
    frame = request_frame(request_id, operation, argument)
    # Configure BEFORE open so this operation does not intentionally reset the board.
    uart = serial.Serial(port=None, baudrate=115200, timeout=0.1, write_timeout=2)
    uart.dtr = False
    uart.rts = False
    uart.port = port
    reader = ResponseReader(request_id)
    try:
        uart.open()
        uart.write(frame)
        uart.flush()
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            value = reader.feed(uart.read(512))
            if value is not None:
                if not isinstance(value, dict):
                    raise RuntimeError('Malformed debug response')
                if value.get('error'):
                    raise RuntimeError(value['error'])
                return {'source': 'device-uart', 'port': port, 'operation': operation,
                        'requestId': request_id, 'result': value}
        # Never retry a click implicitly: it may have executed without a received ACK.
        raise TimeoutError('No hardware UI response; install firmware with USB debug support, '
                           'and close any serial monitor/pairing session. Clicks are not auto-retried.')
    finally:
        uart.close()


def bridge_debug(bridge, operation, argument='', timeout=30):
    from urllib.request import Request, urlopen
    from urllib.parse import urlsplit
    url = urlsplit(bridge)
    if url.scheme != 'http' or url.hostname not in ('127.0.0.1', 'localhost'):
        raise ValueError('Debug bridge must be a local HTTP address')
    data = json.dumps({'operation': operation, 'argument': argument}, ensure_ascii=False).encode('utf-8')
    request = Request(bridge.rstrip('/') + '/debug', data=data, headers={'Content-Type': 'application/json'})
    from urllib.error import HTTPError
    try:
        with urlopen(request, timeout=timeout) as response: return json.load(response)
    except HTTPError as error:
        value = json.load(error)
        raise RuntimeError(value.get('error', str(error))) from error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=OPS)
    parser.add_argument('--port')
    parser.add_argument('--bridge')
    parser.add_argument('--text', default='')
    parser.add_argument('--id', dest='target', default='')
    parser.add_argument('--direction', default='')
    parser.add_argument('--timeout', type=float, default=12)
    args = parser.parse_args()
    if bool(args.port) == bool(args.bridge): parser.error('Use exactly one of --port or --bridge')
    if not 0 < args.timeout <= 60:
        parser.error('--timeout must be in (0, 60]')
    if args.operation == 'tap' and (not args.target or args.direction):
        parser.error('tap requires --id and does not accept --direction')
    if args.operation == 'swipe' and (args.direction not in ('left', 'right', 'up', 'down') or args.target):
        parser.error('swipe requires --direction left/right/up/down and does not accept --id')
    if args.operation in ('screen', 'tree', 'health') and (args.target or args.direction):
        parser.error('screen/tree do not take targets')
    try:
        argument = args.text if args.operation == 'draft' else args.target or args.direction
        value = bridge_debug(args.bridge, args.operation, argument, args.timeout) if args.bridge else debug_device(args.port, args.operation, argument, args.timeout)
        print(json.dumps(value, ensure_ascii=False, indent=2))
    except Exception as error:
        print(f'Device debug failed: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
