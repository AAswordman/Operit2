import importlib.util
import io
import json
import queue
import threading
import unittest
from pathlib import Path
from unittest.mock import patch
from http.server import HTTPServer
from urllib.request import Request, urlopen
from urllib.error import HTTPError

spec = importlib.util.spec_from_file_location('device_session', Path(__file__).parents[1] / 'device-session.py')
bridge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bridge)

class Process:
    def __init__(self, lines):
        self.stdin = io.StringIO(); self.stdout = io.StringIO(''.join(json.dumps(v)+'\n' for v in lines)); self.terminated = False
    def poll(self): return 1 if self.terminated else None
    def terminate(self): self.terminated = True
    def wait(self, timeout=None): return 0

class Tests(unittest.TestCase):
    def test_session_has_one_cli_and_preserves_business_error_snapshots(self):
        process = Process([{'listening': True}, {'connected': False, 'messages': [], 'error': 'offline'}])
        with patch.object(bridge.subprocess, 'Popen', return_value=process) as spawn:
            session = bridge.CoreSession(Path('core-cli'), timeout=1)
            try:
                result = session.plugin('real-board', 'device.ui', 'chat')
                self.assertFalse(result['connected'])
                self.assertEqual(result['error'], 'offline')
                self.assertEqual(spawn.call_count, 1)
                args = json.loads(process.stdin.getvalue().splitlines()[0])
                self.assertEqual(args[:5], ['edge-plugin', 'real-board', 'invoke', 'device.ui', 'chat'])
            finally: session.close()
    def test_timeout_stops_ambiguous_pipeline_without_replaying_mutation(self):
        session = object.__new__(bridge.CoreSession)
        session.process = Process([]); session.lock = threading.Lock(); session.output = queue.Queue(); session.timeout = .01
        with self.assertRaises(TimeoutError): session.command(['edge-plugin', 'real-board', 'invoke', 'device.ui', 'tap'])
        self.assertTrue(session.process.terminated)
        self.assertEqual(len(session.process.stdin.getvalue().splitlines()), 1)
    def test_http_targets_real_node_and_bounds_body_and_actions(self):
        class Session:
            calls = []
            def plugin(self, *args): self.calls.append(args); return {'ok': True}
        session = Session()
        server = HTTPServer(('127.0.0.1', 0), bridge.make_handler(session, 'real-board'))
        worker = threading.Thread(target=server.serve_forever); worker.start()
        url = f'http://127.0.0.1:{server.server_port}/debug'
        try:
            req = Request(url, data=json.dumps({'operation': 'draft', 'argument': 'hello'}).encode(), headers={'Content-Type': 'application/json'})
            with urlopen(req, timeout=2) as response: result = json.load(response)
            self.assertEqual(result['source'], 'device-link-serial')
            self.assertEqual(session.calls, [('real-board', 'device.ui', 'draft', 'hello')])
            for body in [b'x'*8193, b'{"operation":"erase"}']:
                try:
                    with urlopen(Request(url, data=body), timeout=2): self.fail('invalid request accepted')
                except HTTPError as error:
                    self.assertEqual(error.code, 400); error.close()
                except (ConnectionAbortedError, ConnectionResetError): pass
            self.assertEqual(len(session.calls), 1)
        finally:
            server.shutdown(); worker.join(); server.server_close()

    def test_command_bridge_denies_mutation_other_devices_and_browser_requests(self):
        class Session:
            def __init__(self): self.calls = []
            def command(self, args): self.calls.append(args); return {'ok': True}
        session = Session()
        server = HTTPServer(('127.0.0.1', 0), bridge.make_handler(session, 'real-board'))
        worker = threading.Thread(target=server.serve_forever); worker.start()
        url = f'http://127.0.0.1:{server.server_port}/command'
        def post(args, extra=None):
            return urlopen(Request(url, data=json.dumps({'args': args}).encode(),
                headers={'Content-Type': 'application/json', **(extra or {})}), timeout=2)
        try:
            for args in [['peers'], ['space', 'show']]:
                with post(args) as response: self.assertEqual(response.status, 200)
            for args in [['space', 'leave'], ['space', 'show', '--extra'], ['control', 'remove'],
                         ['core', 'model', 'set'], ['edge-plugin', 'other-board', 'invoke'], ['peers', 'delete']]:
                with self.assertRaises(HTTPError) as caught: post(args)
                self.assertEqual(caught.exception.code, 400); caught.exception.close()
            for headers in [{'Origin': 'http://example.com'}, {'Host': 'example.com'}, {'Content-Type': 'text/plain'}]:
                with self.assertRaises(HTTPError) as caught: post(['peers'], headers)
                self.assertEqual(caught.exception.code, 400); caught.exception.close()
            self.assertEqual(session.calls, [['peers'], ['space', 'show']])
        finally:
            server.shutdown(); worker.join(); server.server_close()

if __name__ == '__main__': unittest.main()
