"""Use the unmodified guest Python to validate virtual files and guest state."""
import errno
import json
import mmap
import os
import pathlib
import stat
import threading

assert os.readlink('/proc/self') == str(os.getpid())
assert pathlib.Path('/proc/self/exe').resolve() == pathlib.Path('/usr/bin/python3.12')
assert b'/operit-probes/virtual_filesystem.py\0' in pathlib.Path('/proc/self/cmdline').read_bytes()
assert b'\0' in pathlib.Path('/proc/self/environ').read_bytes()
assert len(pathlib.Path('/proc/self/auxv').read_bytes()) % 16 == 0
assert os.readlink('/proc/self/cwd') == os.getcwd()
assert stat.S_ISCHR(os.stat('/dev/null').st_mode)
assert os.major(os.stat('/dev/null').st_rdev) == 1
assert os.minor(os.stat('/dev/null').st_rdev) == 3
with open('/dev/null', 'r+b', buffering=0) as device:
    assert device.read(17) == b''
    assert device.write(b'discard') == 7
with open('/dev/zero', 'r+b', buffering=0) as device:
    assert device.read(8192) == bytes(8192)
    with mmap.mmap(device.fileno(), 4096, flags=mmap.MAP_PRIVATE) as region:
        assert region[:16] == bytes(16)
        region[:4] = b'zero'
        assert region[:4] == b'zero'
with open('/dev/full', 'wb', buffering=0) as device:
    try:
        device.write(b'x')
    except OSError as error:
        assert error.errno == errno.ENOSPC
    else:
        raise AssertionError('/dev/full accepted data')
with open('/dev/urandom', 'rb', buffering=0) as device:
    assert device.read(32) != bytes(32)
with open('/proc/self/cmdline', 'rb', buffering=0) as source:
    duplicate = os.dup(source.fileno())
    assert source.read(1) + os.read(duplicate, 7) == pathlib.Path('/proc/self/cmdline').read_bytes()[:8]
    os.close(duplicate)
    fdlink = f'/proc/self/fd/{source.fileno()}'
    assert stat.S_ISLNK(os.lstat(fdlink).st_mode)
    assert os.readlink(fdlink).endswith('/cmdline')
    assert pathlib.Path(fdlink).read_bytes() == pathlib.Path('/proc/self/cmdline').read_bytes()
assert 'python3.12' in pathlib.Path('/proc/self/maps').read_text()
assert '[stack]' in pathlib.Path('/proc/self/maps').read_text()
assert {'self', 'thread-self', str(os.getpid())} <= set(os.listdir('/proc'))
assert {'null', 'zero', 'full', 'urandom', 'fd'} <= set(os.listdir('/dev'))
assert {'proc', 'dev'} <= set(os.listdir('/'))
started = threading.Event()
release = threading.Event()
state = {}

def worker():
    state['tid'] = threading.get_native_id()
    state['link'] = os.readlink('/proc/thread-self')
    started.set()
    release.wait(10)

thread = threading.Thread(target=worker)
thread.start()
try:
    assert started.wait(10)
    assert state['link'].endswith('/task/' + str(state['tid']))
    assert str(state['tid']) in os.listdir('/proc/self/task')
    status = pathlib.Path(f"/proc/self/task/{state['tid']}/status").read_text()
    assert f"Pid:\t{state['tid']}\n" in status
finally:
    release.set()
    thread.join(10)
assert not thread.is_alive()
assert str(state['tid']) not in os.listdir('/proc/self/task')
print(json.dumps({'test': 'python-virtual-filesystem', 'ok': True}))
