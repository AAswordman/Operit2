"""Runs as Termux's app user and relays a real Bash PTY to Operit."""

import errno
import fcntl
import os
import pty
import select
import signal
import socket
import struct
import sys
import termios
import time


def connect(port, token, role):
    """Authenticates one connection using the per-session launch token."""
    connection = socket.create_connection(("127.0.0.1", int(port)), timeout=10)
    connection.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
    connection.sendall(token.encode("ascii") + role)
    return connection


def relay(master, data, control, child):
    """Reports child exit independently of PTY EOF, then drains bounded trailing output."""
    pending = bytearray()
    sizes = bytearray()
    status = None
    exited_at = last_output = None
    output_open = True
    os.set_blocking(master, False)
    while True:
        now = time.monotonic()
        if status is None:
            waited, child_status = os.waitpid(child, os.WNOHANG)
            if waited:
                status = child_status
                code = os.waitstatus_to_exitcode(status)
                control.sendall(struct.pack("!i", code if code >= 0 else 128 - code))
                exited_at = last_output = now
        if status is not None and (not output_open or now - last_output >= 0.1 or now - exited_at >= 1):
            return status
        readers = [control] + ([master] if output_open else [])
        if len(pending) < 65536:
            readers.append(data)
        readable, writable, _ = select.select(readers, [master] if pending and output_open else [], [], 0.025)
        if control in readable:
            chunk = control.recv(4096)
            if not chunk:
                return status
            sizes.extend(chunk)
            while len(sizes) >= 4:
                rows, cols = struct.unpack("!HH", sizes[:4])
                del sizes[:4]
                fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        if data in readable:
            chunk = data.recv(16384)
            if not chunk:
                return status
            pending.extend(chunk)
        if master in readable:
            try:
                chunk = os.read(master, 16384)
            except OSError as error:
                if error.errno == errno.EIO:
                    chunk = b""
                else:
                    raise
            if chunk:
                data.sendall(chunk)
                last_output = time.monotonic()
            else:
                output_open = False
        if master in writable:
            try:
                del pending[:os.write(master, pending)]
            except BlockingIOError:
                pass


def run(data, control, rows, cols, prompt, directory, mode, command):
    """Starts interactive Bash and reports its exit after forwarding the final output."""
    child, master = pty.fork()
    if child == 0:
        try:
            fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
            os.chdir(directory)
            os.environ["TERM"] = "xterm-256color"
            if prompt:
                os.environ.update(PROMPT_COMMAND=prompt, PS1="$PWD $ ")
            else:
                os.environ.pop("PROMPT_COMMAND", None)
                os.environ.pop("PS1", None)
            shell = os.path.join(os.environ["PREFIX"], "bin", "bash")
            os.execv(shell, [shell, "--noprofile", "--norc"] + (["-c", command] if mode == "command" else ["-i"]))
        except BaseException as error:
            print(f"Termux Bash startup failed: {error}", file=sys.stderr, flush=True)
            os._exit(126)
    status = None
    try:
        status = relay(master, data, control, child)
    finally:
        os.close(master)
        if status is None:
            try:
                os.kill(child, signal.SIGKILL)
            except ProcessLookupError:
                pass
            os.waitpid(child, 0)


if __name__ == "__main__":
    port, token, rows, cols, prompt, directory, mode, command = sys.argv[1:]
    with connect(port, token, b"D") as data, connect(port, token, b"C") as control:
        run(data, control, int(rows), int(cols), prompt, directory, mode, command)
