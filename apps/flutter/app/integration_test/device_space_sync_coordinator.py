#!/usr/bin/env python3
"""Coordinate the two native Flutter tests and capture their actual windows."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--simulator', default='booted')
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
state = {}
lock = threading.Lock()

# Select the largest application window, excluding popovers and other apps.
with tempfile.TemporaryDirectory(prefix='operit-sync-capture-') as temporary:
    window_tool = Path(temporary) / 'window'
    swift = window_tool.with_suffix('.swift')
    swift.write_text('''import Foundation
import CoreGraphics
import AppKit
let app = NSRunningApplication.runningApplications(withBundleIdentifier: "com.ai.assistance.operit2")
 .first { $0.bundleURL?.path.contains("/build/macos/") == true }
if CommandLine.arguments.contains("--focus") {
 app?.activate(options: [.activateAllWindows])
}
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
let candidates = windows.filter { ($0[kCGWindowOwnerPID as String] as? Int32) == app?.processIdentifier }
let ordered = candidates.sorted { a,b in
 let x = a[kCGWindowBounds as String] as? [String: Double] ?? [:]
 let y = b[kCGWindowBounds as String] as? [String: Double] ?? [:]
 return (x["Width"] ?? 0)*(x["Height"] ?? 0) > (y["Width"] ?? 0)*(y["Height"] ?? 0)
}
if let window = ordered.first { print(window[kCGWindowNumber as String]!) }
''')
    subprocess.run(['swiftc', str(swift), '-o', str(window_tool)], check=True)

    def save_state():
        (args.output / 'native-events.json').write_text(
            json.dumps(state, ensure_ascii=False, indent=2))

    def capture(name):
        try:
            path = args.output.resolve() / f'{name}.png'
            if name.startswith('ios-'):
                command = ['xcrun', 'simctl', 'io', args.simulator, 'screenshot', str(path)]
            else:
                subprocess.run([str(window_tool), '--focus'], check=True, capture_output=True)
                window = subprocess.check_output([str(window_tool)], text=True).strip()
                if not window:
                    raise RuntimeError('Operit2 has no visible application window')
                command = ['screencapture', '-x', '-l', window, str(path)]
            subprocess.run(command, check=True, capture_output=True)
            with lock:
                state[f'captured-{name}'] = str(path)
                save_state()
            print('SCREENSHOT', name, flush=True)
        except Exception as error:
            print('SCREENSHOT_ERROR', name, repr(error), flush=True)

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            with lock:
                value = dict(state) if self.path == '/state' else state.get(self.path[1:])
            self.send_response(200)
            self.end_headers()
            self.wfile.write(json.dumps(value).encode())

        def do_PUT(self):
            key = self.path[1:]
            value = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            with lock:
                state[key] = value
                save_state()
            print(time.strftime('%H:%M:%S'), key, str(value)[:240], flush=True)
            subprocess.run([str(window_tool), '--focus'], capture_output=True)
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'true')
            if key.startswith('capture-'):
                threading.Thread(target=capture, args=(key[8:],), daemon=True).start()

    ThreadingHTTPServer(('127.0.0.1', 39180), Handler).serve_forever()
