#!/usr/bin/env python3
"""Editor-owned real-device debug bridge; Core remains an independent process.
One long-lived CLI session owns the UART. Never open a monitor/debug UART beside it.
"""
import argparse
import json
import queue
import subprocess
import sys
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

class CoreSession:
    def __init__(self, cli, timeout=30):
        self.timeout = timeout
        self.lock = threading.Lock()
        self.output = queue.Queue(maxsize=32)
        self.process = subprocess.Popen([str(cli), 'cli', '--json', 'link', 'session', 'tcp',
            '--bind', '127.0.0.1:37195', '--no-discovery'], cwd=Path(__file__).resolve().parents[2],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            encoding='utf-8', errors='replace', bufsize=1)
        def read():
            for line in self.process.stdout:
                try: value = json.loads(line)
                except ValueError:
                    if 'RuntimePeerService' in line and ('revoked' in line or 'failed' in line or 'offline' in line):
                        print(line.rstrip(), file=sys.stderr, flush=True)
                    continue
                try: self.output.put(value, timeout=1)
                except queue.Full: self.process.terminate(); break
            try: self.output.put({'error': 'Core session exited'}, timeout=1)
            except queue.Full: pass
        threading.Thread(target=read, daemon=True).start()
        ready = self.output.get(timeout=timeout)
        if not isinstance(ready, dict) or not ready.get('listening'):
            self.close()
            raise RuntimeError(f'Core session did not start: {ready}')
    def command(self, args):
        with self.lock:
            if self.process.poll() is not None: raise RuntimeError('Core session exited')
            self.process.stdin.write(json.dumps(args, ensure_ascii=False) + '\n')
            self.process.stdin.flush()
            try: value = self.output.get(timeout=self.timeout)
            except queue.Empty:
                # Correlation is now ambiguous: do not retry a mutation or allow
                # a late result to be mistaken for a subsequent command.
                self.process.terminate()
                raise TimeoutError('Core response deadline reached; session stopped without retry')
            if isinstance(value, dict) and set(value) == {'error'} and value['error']: raise RuntimeError(value['error'])
            return value
    def plugin(self, node, plugin, action, argument=''):
        return self.command(['edge-plugin', node, 'invoke', plugin, action,
            json.dumps({'argument': argument}, ensure_ascii=False)])
    def close(self):
        if self.process.poll() is None:
            try: self.process.stdin.write('["quit"]\n'); self.process.stdin.flush(); self.process.wait(timeout=8)
            except (OSError, subprocess.TimeoutExpired): self.process.terminate(); self.process.wait(timeout=5)
        for stream in (self.process.stdin, self.process.stdout):
            if stream: stream.close()

def make_handler(session, node):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args): pass
        def do_POST(self):
            try:
                # This is a local CLI bridge, not a browser API. Reject cross-
                # origin requests/DNS rebinding before dispatching any action.
                if self.headers.get('Origin') is not None:
                    raise ValueError('Browser-origin requests are not supported')
                if self.headers.get('Host') != f'127.0.0.1:{self.server.server_port}':
                    raise ValueError('Expected loopback Host')
                if self.headers.get_content_type() != 'application/json':
                    raise ValueError('Expected application/json')
                size = int(self.headers.get('Content-Length', '0'))
                if not 0 < size <= 8192: raise ValueError('Invalid/bounded request body required')
                body = json.loads(self.rfile.read(size))
                if not isinstance(body, dict): raise ValueError('Expected JSON object')
                if self.path == '/stop':
                    result = {'stopped': True}
                    threading.Thread(target=self.server.shutdown, daemon=True).start()
                elif self.path == '/debug':
                    action = body.get('operation')
                    if action not in ('screen', 'tree', 'tap', 'swipe', 'draft', 'chat', 'health', 'status'):
                        raise ValueError('Unsupported operation')
                    plugin = 'device.status' if action in ('health', 'status') else 'device.ui'
                    action = 'read' if action == 'status' else action
                    value = session.plugin(node, plugin, action, body.get('argument', ''))
                    result = {'source': 'device-link-serial', 'nodeId': node, 'result': value}
                elif self.path == '/command':
                    args = body.get('args')
                    if not isinstance(args, list) or not args or not all(isinstance(v, str) for v in args):
                        raise ValueError('Expected command argument array')
                    # Diagnostics only. UI mutations are explicitly enumerated
                    # by /debug and always bound to this session's device.
                    if args not in (['peers'], ['space', 'show']):
                        raise ValueError('Only peers and space show diagnostics are allowed')
                    result = {'result': session.command(args)}
                else: raise ValueError('Unknown endpoint')
                status = 200
            except Exception as error:
                status, result = 400, {'error': str(error)}
            payload = json.dumps(result, ensure_ascii=False).encode('utf-8')
            self.send_response(status); self.send_header('Content-Type', 'application/json; charset=utf-8')
            self.send_header('Content-Length', str(len(payload))); self.end_headers(); self.wfile.write(payload)
    return Handler

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--node', required=True)
    parser.add_argument('--listen', type=int, default=18766)
    default = Path(__file__).resolve().parents[2] / 'apps/cli/target/release' / ('operit2.exe' if sys.platform == 'win32' else 'operit2')
    parser.add_argument('--cli', type=Path, default=default)
    args = parser.parse_args()
    if not 1 <= args.listen <= 65535: parser.error('Invalid local port')
    session = CoreSession(args.cli)
    try:
        server = HTTPServer(('127.0.0.1', args.listen), make_handler(session, args.node))
        print(json.dumps({'ready': True, 'bridge': f'http://127.0.0.1:{args.listen}', 'nodeId': args.node}), flush=True)
        try: server.serve_forever()
        except KeyboardInterrupt: pass
        finally: server.server_close()
    finally: session.close()
if __name__ == '__main__': main()
