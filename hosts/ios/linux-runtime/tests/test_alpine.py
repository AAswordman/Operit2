"""Integration tests against the pinned, unmodified Alpine AArch64 userspace."""
import argparse
from http.server import BaseHTTPRequestHandler, HTTPServer
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import threading
import unittest

parser = argparse.ArgumentParser()
parser.add_argument('--runtime', required=True)
parser.add_argument('--root', required=True)
parser.add_argument('--fixed-tests', required=True)
OPTIONS, EXTRA = parser.parse_known_args()


class AlpineTests(unittest.TestCase):
    def run_busybox(self, *args, accelerated=False, root=None):
        command = [OPTIONS.runtime, '--root', str(root or OPTIONS.root), '--stats']
        if accelerated:
            command.append('--fixed-acceleration')
        result = subprocess.run([*command, '--', '/bin/busybox', *args], capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        stats = json.loads(result.stderr.decode().splitlines()[-1])
        self.assertEqual(stats['unsupported_syscalls'], 0)
        self.assertEqual(stats['status'], 0)
        return result.stdout, stats

    def test_uname_reports_guest_identity_without_invented_kernel_metadata(self):
        with tempfile.TemporaryDirectory(prefix='arm64-uname-') as temporary:
            root = Path(temporary) / 'root'
            shutil.copytree(OPTIONS.root, root, symlinks=True)
            hostname = root / 'etc/hostname'
            hostname.unlink(missing_ok=True)
            hostname.write_text('alpine-test\n')
            for accelerated in (False, True):
                output, _ = self.run_busybox('uname', '-snrvm', accelerated=accelerated, root=root)
                self.assertEqual(output, b'Linux alpine-test unknown unknown aarch64\n')
            hostname.unlink()
            output, _ = self.run_busybox('uname', '-n', root=root)
            self.assertEqual(output, b'(none)\n')

    def test_apk_update_publishes_downloaded_index(self):
        # Serve a valid empty v2 repository locally: no CDN/DNS dependency, and
        # no installation or changes to the supplied rootfs are needed.
        archive = io.BytesIO()
        with tarfile.open(fileobj=archive, mode='w:gz') as repository:
            for name, contents in [('DESCRIPTION', b'runtime apk regression\n'),
                                   ('APKINDEX', b'')]:
                member = tarfile.TarInfo(name)
                member.size = len(contents)
                repository.addfile(member, io.BytesIO(contents))
        payload = archive.getvalue()
        requests = []

        class RepositoryHandler(BaseHTTPRequestHandler):
            def do_GET(self):
                requests.append(self.path)
                if self.path != '/aarch64/APKINDEX.tar.gz':
                    self.send_error(404)
                    return
                self.send_response(200)
                self.send_header('Content-Length', str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

            def log_message(self, *args):
                pass

        with tempfile.TemporaryDirectory(prefix='arm64-apk-update-') as temporary:
            root = Path(temporary) / 'root'
            shutil.copytree(OPTIONS.root, root, symlinks=True)
            cache = root / 'var/cache/apk/test-update'
            cache.mkdir(parents=True)
            with HTTPServer(('127.0.0.1', 0), RepositoryHandler) as server:
                repositories = root / 'etc/apk/test-repositories'
                repositories.write_text(f'http://127.0.0.1:{server.server_port}\n')
                worker = threading.Thread(target=server.serve_forever, daemon=True)
                worker.start()
                try:
                    result = subprocess.run([
                        OPTIONS.runtime, '--root', str(root), '--fixed-acceleration',
                        '--stats', '--', '/sbin/apk', '--allow-untrusted',
                        '--repositories-file', '/etc/apk/test-repositories',
                        '--cache-dir', '/var/cache/apk/test-update', '--no-progress',
                        '--timeout', '5', 'update',
                    ], capture_output=True, timeout=20,
                        env={**os.environ, 'NO_PROXY': '127.0.0.1',
                             'no_proxy': '127.0.0.1'})
                finally:
                    server.shutdown()
                    worker.join(timeout=5)
            self.assertEqual(result.returncode, 0, (result.stdout + result.stderr).decode())
            self.assertIn('/aarch64/APKINDEX.tar.gz', requests)
            indexes = list(cache.glob('APKINDEX.*.tar.gz'))
            self.assertEqual(len(indexes), 1, list(cache.iterdir()))
            self.assertEqual(indexes[0].read_bytes(), payload)
            self.assertFalse(list(cache.glob('*.tmp*')), list(cache.iterdir()))
            stats = json.loads(result.stderr.decode().splitlines()[-1])
            self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_unmodified_dynamic_alpine_printf(self):
        for accelerated in (False, True):
            output, stats = self.run_busybox('printf', 'alpine-arm64\\n', accelerated=accelerated)
            self.assertEqual(output, b'alpine-arm64\n')
            self.assertEqual(stats['accelerated_calls'] > 0, accelerated)

    def test_dynamic_shell_builtins_and_loop(self):
        script = 'printf "arm64-shell\\n"; pwd; i=0; while [ "$i" -lt 3 ]; do echo "$i"; i=$((i+1)); done'
        for accelerated in (False, True):
            output, _ = self.run_busybox('sh', '-c', script, accelerated=accelerated)
            self.assertEqual(output, b'arm64-shell\n/\n0\n1\n2\n')

    def test_directory_open_and_linux_dirent_layout(self):
        reference, _ = self.run_busybox('ls', '-1', '/bin')
        accelerated, _ = self.run_busybox('ls', '-1', '/bin', accelerated=True)
        self.assertIn(b'busybox', reference.splitlines())
        self.assertIn(b'sh', reference.splitlines())
        self.assertEqual(reference, accelerated)

    def test_file_reads_and_root_symlinks(self):
        reference, _ = self.run_busybox('cat', '/etc/alpine-release')
        accelerated, _ = self.run_busybox('cat', '/etc/alpine-release', accelerated=True)
        self.assertEqual(reference.strip(), b'3.23.6')
        self.assertEqual(reference, accelerated)

    def test_fixed_helper_boundaries_identity_and_mutation(self):
        result = subprocess.run([OPTIONS.fixed_tests, str(Path(OPTIONS.root).resolve())], capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0, result.stderr.decode())


if __name__ == '__main__':
    unittest.main(argv=[__file__, *EXTRA], verbosity=2)
