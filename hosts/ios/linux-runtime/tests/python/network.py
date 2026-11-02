"""Real guest sockets, readiness, threaded blocking I/O, and descriptor passing."""
import array
import errno
import json
import os
import pathlib
import select
import socket
import tempfile
import threading
import time
import struct

left, right = socket.socketpair()
with left, right:
    left.sendall(b'pair-data')
    assert select.select([right], [], [], 1)[0] == [right]
    assert right.recv(9) == b'pair-data'
    poll = select.poll()
    poll.register(right, select.POLLIN)
    assert poll.poll(0) == []
    left.sendmsg([b'vector-', b'message'])
    data, ancillary, flags, address = right.recvmsg(32)
    assert data == b'vector-message' and not ancillary
    with open('/etc/alpine-release', 'rb', buffering=0) as source:
        left.sendmsg([b'F'], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', [source.fileno()]))])
        data, ancillary, flags, address = right.recvmsg(1, socket.CMSG_SPACE(4), socket.MSG_CMSG_CLOEXEC)
        assert data == b'F' and len(ancillary) == 1
        descriptors = array.array('i')
        descriptors.frombytes(ancillary[0][2][:4])
        received = descriptors[0]
        try:
            assert not os.get_inheritable(received)
            first = os.read(received, 2)
            assert source.read(2) == pathlib.Path('/etc/alpine-release').read_bytes()[2:4]
            assert first == b'3.'
        finally:
            os.close(received)
    with open('/etc/alpine-release', 'rb', buffering=0) as source:
        left.sendmsg([b'D'], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', [source.fileno()]))])
        assert right.recv(1) == b'D'  # Drop this message's descriptor intentionally.
    counter = os.eventfd(7, 0)
    try:
        left.sendmsg([b'E'], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', [counter]))])
        os.close(counter)
        counter = -1
        data, ancillary, flags, address = right.recvmsg(1, socket.CMSG_SPACE(4))
        received = array.array('i', ancillary[0][2][:4])[0]
        try:
            assert data == b'E' and os.eventfd_read(received) == 7
        finally:
            os.close(received)
    finally:
        if counter >= 0:
            os.close(counter)
    right.settimeout(0.02)
    try:
        right.recv(1)
    except TimeoutError:
        pass
    else:
        raise AssertionError('socket timeout did not expire')

left, right = socket.socketpair()
with left, right:
    def fragments():
        left.sendall(b'ab')
        time.sleep(0.01)
        left.sendall(b'cde')
        time.sleep(0.01)
        left.sendall(b'hi')
        left.shutdown(socket.SHUT_WR)
    producer = threading.Thread(target=fragments)
    producer.start()
    assert right.recv(5, socket.MSG_WAITALL) == b'abcde'
    assert right.recv(10, socket.MSG_WAITALL) == b'hi'
    producer.join(5)
    assert not producer.is_alive()
left, right = socket.socketpair()
with left, right:
    left.sendall(b'ab')
    right.setsockopt(socket.SOL_SOCKET, socket.SO_RCVTIMEO, struct.pack('<qq', 0, 20000))
    assert right.recv(5, socket.MSG_WAITALL) == b'ab'
left, right = socket.socketpair()
with left, right:
    left.sendall(b'vectors')
    one, two = bytearray(3), bytearray(4)
    assert right.recvmsg_into([one, two], 0, socket.MSG_WAITALL)[0] == 7
    assert one + two == b'vectors'

server = socket.socket()
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind(('127.0.0.1', 0))
server.listen(4)
port = server.getsockname()[1]
errors = []

def serve():
    try:
        with server:
            stream, peer = server.accept()
            with stream:
                assert peer[0] == '127.0.0.1'
                assert stream.recv(32) == b'guest-client'
                stream.sendall(b'guest-server')
                stream.shutdown(socket.SHUT_WR)
    except BaseException as error:
        errors.append(repr(error))

thread = threading.Thread(target=serve)
thread.start()
with socket.create_connection(('127.0.0.1', port), timeout=5) as client:
    client.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
    assert client.getsockopt(socket.SOL_SOCKET, socket.SO_ERROR) == 0
    client.sendall(b'guest-client')
    assert client.recv(32) == b'guest-server'
    assert client.recv(1) == b''
thread.join(5)
assert not thread.is_alive() and not errors, errors

with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as server, socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as client:
    server.bind(('127.0.0.1', 0))
    client.sendto(b'datagram', server.getsockname())
    data, source = server.recvfrom(32)
    assert data == b'datagram'
    server.sendto(b'reply', source)
    assert client.recvfrom(32)[0] == b'reply'
    client.sendto(b'x' * 100, server.getsockname())
    data = bytearray(7)
    assert server.recvfrom_into(data, 7, socket.MSG_TRUNC)[0] == 100
    assert data == b'x' * 7

with tempfile.TemporaryDirectory() as directory:
    name = directory + '/endpoint'
    with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as server, socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as client:
        server.bind(name)
        assert server.getsockname() == name
        client.sendto(b'unix', name)
        assert server.recv(16) == b'unix'
    os.unlink(name)

abstract = b'\0operit-abstract-network-probe'
with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as server, socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as client:
    server.bind(abstract)
    assert server.getsockname() == abstract
    client.sendto(b'abstract', abstract)
    assert server.recv(16) == b'abstract'
with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as rebound:
    rebound.bind(abstract)
    assert rebound.getsockname() == abstract

# A real refused connection must preserve the Linux network errno.
with socket.socket() as reserve:
    reserve.bind(('127.0.0.1', 0))
    unavailable = reserve.getsockname()[1]
with socket.socket() as client:
    client.settimeout(1)
    assert client.connect_ex(('127.0.0.1', unavailable)) == errno.ECONNREFUSED
print(json.dumps({'test': 'python-network', 'ok': True}))
