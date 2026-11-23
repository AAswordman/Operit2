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


def connect(port, token, role):
    """Authenticates one connection using the per-session launch token."""
    connection = socket.create_connection(("127.0.0.1", int(port)), timeout=10)
    connection.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
    connection.sendall(token.encode("ascii") + role)
    return connection


def relay(master, data, control):
    """Relays binary output and input while independently handling PTY resizing."""
    pending = bytearray()
    sizes = bytearray()
    os.set_blocking(master, False)
    while True:
        readers = [master, control]
        if len(pending) < 65536:
            readers.append(data)
        readable, writable, _ = select.select(readers, [master] if pending else [], [])
        if control in readable:
            chunk = control.recv(4096)
            if not chunk:
                return False
            sizes.extend(chunk)
            while len(sizes) >= 4:
                rows, cols = struct.unpack("!HH", sizes[:4])
                del sizes[:4]
                fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        if data in readable:
            chunk = data.recv(16384)
            if not chunk:
                return False
            pending.extend(chunk)
        if master in readable:
            try:
                chunk = os.read(master, 16384)
            except OSError as error:
                if error.errno == errno.EIO:
                    return True
                raise
            if not chunk:
                return True
            data.sendall(chunk)
        if master in writable:
            try:
                del pending[:os.write(master, pending)]
            except BlockingIOError:
                pass


def run(data, control, rows, cols, prompt, directory):
    """Starts interactive Bash and reports its exit after forwarding the final output."""
    child, master = pty.fork()
    if child == 0:
        try:
            fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
            os.chdir(directory)
            os.environ.update(TERM="xterm-256color", PROMPT_COMMAND=prompt, PS1="$PWD $ ")
            shell = os.path.join(os.environ["PREFIX"], "bin", "bash")
            os.execv(shell, [shell, "--noprofile", "--norc", "-i"])
        except BaseException as error:
            print(f"Termux Bash startup failed: {error}", file=sys.stderr, flush=True)
            os._exit(126)
    finished = False
    try:
        finished = relay(master, data, control)
    finally:
        os.close(master)
        if not finished:
            try:
                os.kill(child, signal.SIGKILL)
            except ProcessLookupError:
                pass
        _, status = os.waitpid(child, 0)
    code = os.waitstatus_to_exitcode(status)
    control.sendall(struct.pack("!i", code if code >= 0 else 128 - code))


if __name__ == "__main__":
    port, token, rows, cols, prompt, directory = sys.argv[1:]
    with connect(port, token, b"D") as data, connect(port, token, b"C") as control:
        run(data, control, int(rows), int(cols), prompt, directory)
