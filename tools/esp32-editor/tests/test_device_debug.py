import importlib.util
from pathlib import Path
import json
import struct
import unittest
import zlib

spec = importlib.util.spec_from_file_location('device_debug', Path(__file__).resolve().parents[1] / 'device-debug.py')
debug = importlib.util.module_from_spec(spec)
spec.loader.exec_module(debug)


def response(request_id, value):
    payload = struct.pack('>I', request_id) + json.dumps(value).encode('utf-8')
    return b'OPD2' + struct.pack('>I', len(payload)) + payload + struct.pack('>I', zlib.crc32(payload))


class DeviceDebugTests(unittest.TestCase):
    def test_request_format_and_limits(self):
        frame = debug.request_frame(123, 'tap', 'button:approve')
        self.assertEqual(frame[:4], b'OPD1')
        self.assertEqual(frame[8:13], struct.pack('>IB', 123, 3))
        self.assertEqual(struct.unpack('>I', frame[-4:])[0], zlib.crc32(frame[8:-4]))
        for op, arg in [('nope', ''), ('screen', 'x'), ('tap', ''), ('tap', 'x'*124), ('tap', '\0')]:
            with self.assertRaises(ValueError): debug.request_frame(1, op, arg)

    def test_health_is_bounded_read_only_and_has_its_own_opcode(self):
        frame = debug.request_frame(17, 'health')
        self.assertEqual(frame[8:13], struct.pack('>IB', 17, 5))
        with self.assertRaises(ValueError): debug.request_frame(17, 'health', 'x')
        self.assertEqual(debug.ResponseReader(17).feed(response(17, {
            'heapFreeBytes': 23000, 'uptimeMs': 1200})), {
            'heapFreeBytes': 23000, 'uptimeMs': 1200})

    def test_split_noise_and_wrong_id(self):
        reader = debug.ResponseReader(99)
        self.assertIsNone(reader.feed(b'I log\r\n' + response(123, {'ignored': True})))
        result = None
        for byte in response(99, {'pairingCode': '123456', 'page': 'pairing'}):
            value = reader.feed(bytes([byte]))
            if value is not None: result = value
        self.assertEqual(result['pairingCode'], '123456')

    def test_bad_crc_and_oversized_noise(self):
        reader = debug.ResponseReader(1)
        frame = bytearray(response(1, {'bad': True})); frame[-1] ^= 1
        data = b'OPD2' + struct.pack('>I', 0xffffffff) + frame + response(1, {'ok': True})
        self.assertEqual(reader.feed(data), {'ok': True})

    def test_bounded_log_buffer(self):
        reader = debug.ResponseReader(1)
        for _ in range(100): reader.feed(b'x'*512)
        self.assertLessEqual(len(reader.pending), 3)

    def test_hardware_access_does_not_reset_and_always_closes(self):
        from unittest.mock import patch
        class Uart:
            def __init__(self, **kwargs):
                self.dtr = self.rts = True
                self.closed = False
                self.sent = b''
            def open(self):
                self_case.assertFalse(self.dtr); self_case.assertFalse(self.rts)
            def write(self, frame): self.sent = frame
            def flush(self): pass
            def read(self, length):
                request_id = struct.unpack('>I', self.sent[8:12])[0]
                return response(request_id, {'page':'pairing','pairingCode':'654321'})
            def close(self): self.closed = True
        self_case = self
        uart = Uart()
        with patch('serial.Serial', return_value=uart):
            value = debug.debug_device('COM_TEST', 'screen')
        self.assertTrue(uart.closed)
        self.assertEqual(value['source'], 'device-uart')
        self.assertEqual(value['result']['pairingCode'], '654321')

    def test_failed_click_is_not_retried(self):
        from unittest.mock import patch
        class Uart:
            writes = 0
            closed = False
            def open(self): pass
            def write(self, frame): self.writes += 1
            def flush(self): pass
            def read(self, length): return b''
            def close(self): self.closed = True
        uart = Uart()
        with patch('serial.Serial', return_value=uart):
            with self.assertRaises(TimeoutError): debug.debug_device('COM_TEST', 'tap', 'button:test', timeout=0.01)
        self.assertEqual(uart.writes, 1)
        self.assertTrue(uart.closed)


if __name__ == '__main__': unittest.main()
