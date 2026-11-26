"""Exercise the real Android Termux helper and browser guest PTY relay on POSIX."""

import os
from pathlib import Path
import select
import socket
import struct
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(os.name == "posix", "PTY relay execution requires a POSIX host")
class TermuxProcessTests(unittest.TestCase):
    """Keep process exit independent of socket EOF and shell prompt behavior."""

    def execute(self, command, input_bytes=b""):
        """Launch the production helper with real authenticated loopback sockets."""
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            listener.listen(2)
            listener.settimeout(5)
            token = "a" * 32
            process = subprocess.Popen([
                sys.executable, "-I", "-S", str(ROOT / "hosts/android/src/terminal_termux.py"),
                str(listener.getsockname()[1]), token, "24", "80", "", "/tmp", "command", command,
            ], env={**os.environ, "PREFIX": "/usr"}, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            streams = {}
            try:
                for _ in range(2):
                    stream, _ = listener.accept()
                    stream.settimeout(5)
                    hello = bytearray()
                    while len(hello) < 33:
                        hello.extend(stream.recv(33 - len(hello)))
                    self.assertEqual(hello[:32], token.encode("ascii"))
                    streams[chr(hello[32])] = stream
                if input_bytes:
                    streams["D"].sendall(input_bytes)
                status = bytearray()
                while len(status) < 4:
                    status.extend(streams["C"].recv(4 - len(status)))
                output = bytearray()
                while chunk := streams["D"].recv(16384):
                    output.extend(chunk)
                self.assertEqual(process.wait(timeout=5), 0, process.stderr.read())
                return struct.unpack("!i", status)[0], bytes(output)
            finally:
                for stream in streams.values():
                    stream.close()
                if process.poll() is None:
                    process.kill()
                process.communicate(timeout=5)

    def test_command_without_enter_and_input_driven_command(self):
        """Complete scripts start directly and can still receive input after launch."""
        code, output = self.execute("read -r answer; printf 'answer:%s' \"$answer\"; exit 7", b"hello\n")
        self.assertEqual(code, 7)
        self.assertIn(b"answer:hello", output)

    def test_silence_syntax_and_output_redirection(self):
        """No prompt or stdout marker is needed to return the actual status."""
        for script, expected in [("exit 9", 9), ("if then", 2), ("exec >/dev/null 2>&1; exit 3", 3)]:
            self.assertEqual(self.execute(script)[0], expected)

    def test_descendant_open_output_does_not_hold_process_completion(self):
        """A descendant retaining the slave cannot turn parent exit into an endless wait."""
        started = time.monotonic()
        self.assertEqual(self.execute("sleep 10 & exit 11")[0], 11)
        self.assertLess(time.monotonic() - started, 3)


@unittest.skipUnless(os.name == "posix", "Guest relay execution requires a POSIX host")
class BrowserProcessTests(unittest.TestCase):
    """Run the production C relay against real PTYs before packaging a guest image."""

    @classmethod
    def setUpClass(cls):
        """Compile only the small guest relay into an isolated temporary directory."""
        cls.directory = tempfile.TemporaryDirectory(prefix="operit-relay-")
        cls.executable = Path(cls.directory.name) / "relay"
        subprocess.run(["gcc", "-Wall", "-Wextra", "-Werror", str(ROOT / "tools/web-runtime/operit-runtime-terminal.c"),
                        "-o", str(cls.executable), "-lutil"], check=True)

    @classmethod
    def tearDownClass(cls):
        """Remove the test-owned relay binary and launch files."""
        cls.directory.cleanup()

    def execute(self, script, input_bytes=b""):
        """Use a serial PTY and a guest launch file, preserving the real relay protocol."""
        import pty
        script_path = Path(self.directory.name) / "command.sh"
        script_path.write_text(script, encoding="utf-8")
        master, slave = pty.openpty()
        process = subprocess.Popen([str(self.executable), "24", "80", str(script_path), "test-token"],
                                   stdin=slave, stdout=slave, stderr=slave)
        os.close(slave)
        output = bytearray()
        sent = False
        deadline = time.monotonic() + 5
        try:
            while time.monotonic() < deadline:
                if not select.select([master], [], [], 0.1)[0]:
                    continue
                try:
                    chunk = os.read(master, 16384)
                except OSError:
                    break
                output.extend(chunk)
                if not sent and b"OPERIT_PROCESS_READY\x07" in output:
                    if input_bytes:
                        os.write(master, input_bytes)
                    sent = True
                if b"OPERIT_PROCESS_EXIT;test-token;" in output and output.endswith(b"\x07"):
                    break
            return process.wait(timeout=2), bytes(output)
        finally:
            os.close(master)
            if process.poll() is None:
                process.kill()
            process.wait(timeout=2)

    def test_complete_script_and_interactive_input(self):
        """The relay owns the status while the launched process can read terminal input."""
        code, output = self.execute("read -r answer; printf 'answer:%s' \"$answer\"; exit 7", b"hello\n")
        self.assertEqual(code, 7)
        self.assertIn(b"answer:hello", output)
        self.assertIn(b"OPERIT_PROCESS_EXIT;test-token;7\x07", output)

    def test_syntax_silence_and_held_output(self):
        """Only waitpid decides completion, including when descendants retain PTY handles."""
        for script, expected in [("exit 9", 9), ("if then", 2), ("sleep 10 & exit 11", 11)]:
            code, output = self.execute(script)
            self.assertEqual(code, expected)
            self.assertIn(f"OPERIT_PROCESS_EXIT;test-token;{expected}\x07".encode(), output)


if __name__ == "__main__":
    unittest.main()
