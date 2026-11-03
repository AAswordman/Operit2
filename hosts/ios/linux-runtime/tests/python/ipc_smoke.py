"""Exercise the guest's libc IPC APIs across a real Python fork."""
import ctypes
import os
import sys

libc = ctypes.CDLL(None, use_errno=True)


def checked(value, operation):
    if value == -1:
        raise OSError(ctypes.get_errno(), operation)
    return value


class Message(ctypes.Structure):
    _fields_ = [('kind', ctypes.c_long), ('text', ctypes.c_char * 16)]


class SemaphoreOperation(ctypes.Structure):
    _fields_ = [('number', ctypes.c_ushort), ('operation', ctypes.c_short),
                ('flags', ctypes.c_short)]


libc.shmat.restype = ctypes.c_void_p
libc.shmat.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_int]
libc.shmdt.argtypes = [ctypes.c_void_p]
libc.mq_open.argtypes = [ctypes.c_char_p, ctypes.c_int, ctypes.c_uint, ctypes.c_void_p]
libc.mq_send.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_uint]
libc.mq_receive.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_size_t,
                            ctypes.POINTER(ctypes.c_uint)]
libc.mq_receive.restype = ctypes.c_ssize_t
libc.msgrcv.restype = ctypes.c_ssize_t

message_queue = semaphore = shared_memory = posix_queue = -1
address = None
name = b'/operit-ipc-real-smoke'
try:
    message_queue = checked(libc.msgget(0, 0o1600), 'msgget')
    semaphore = checked(libc.semget(0, 1, 0o1600), 'semget')
    shared_memory = checked(libc.shmget(0, 4096, 0o1600), 'shmget')
    address = libc.shmat(shared_memory, None, 0)
    if address == ctypes.c_void_p(-1).value:
        raise OSError(ctypes.get_errno(), 'shmat')
    cell = ctypes.c_int.from_address(address)
    cell.value = 7
    checked(libc.semctl(semaphore, 0, 16, 0), 'semctl SETVAL')
    posix_queue = checked(libc.mq_open(name, os.O_CREAT | os.O_EXCL | os.O_RDWR, 0o600, None),
                          'mq_open')
    message = Message(7, b'hello')
    checked(libc.msgsnd(message_queue, ctypes.byref(message), 5, 0), 'msgsnd')
    payload = ctypes.create_string_buffer(b'queue')
    checked(libc.mq_send(posix_queue, payload, 5, 2), 'mq_send')

    child = os.fork()
    if child == 0:
        try:
            received = Message()
            size = checked(libc.msgrcv(message_queue, ctypes.byref(received), 16, 0, 0),
                           'child msgrcv')
            assert size == 5 and received.kind == 7 and received.text == b'hello'
            buffer = ctypes.create_string_buffer(8192)
            priority = ctypes.c_uint()
            size = checked(libc.mq_receive(posix_queue, buffer, len(buffer), ctypes.byref(priority)),
                           'child mq_receive')
            assert size == 5 and buffer.raw[:5] == b'queue' and priority.value == 2
            assert cell.value == 7
            cell.value = 23
            operation = SemaphoreOperation(0, 1, 0)
            checked(libc.semop(semaphore, ctypes.byref(operation), 1), 'child semop')
            os._exit(0)
        except BaseException as error:
            print('child-error:', repr(error), file=sys.stderr, flush=True)
            os._exit(111)

    operation = SemaphoreOperation(0, -1, 0)
    checked(libc.semop(semaphore, ctypes.byref(operation), 1), 'parent semop')
    _, status = os.waitpid(child, 0)
    assert status == 0 and cell.value == 23, (status, cell.value)
    print('python-musl-ipc-cross-process-ok')
finally:
    if posix_queue != -1:
        libc.mq_close(posix_queue)
        libc.mq_unlink(name)
    if address is not None and address != ctypes.c_void_p(-1).value:
        libc.shmdt(address)
    if shared_memory != -1:
        libc.shmctl(shared_memory, 0, None)
    if semaphore != -1:
        libc.semctl(semaphore, 0, 0, 0)
    if message_queue != -1:
        libc.msgctl(message_queue, 0, None)
