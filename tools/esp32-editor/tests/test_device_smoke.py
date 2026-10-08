import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('device_smoke', Path(__file__).resolve().parents[1] / 'device-smoke.py')
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)


def health(uptime=100, heap=24000, low=14000):
    return {'uptimeMs': uptime, 'heapFreeBytes': heap, 'heapMinimumFreeBytes': low,
            'largest8bitBlockBytes': 8000, 'currentTaskStackHighWaterMark': 1500}


class DeviceSmokeTests(unittest.TestCase):
    def test_reboots_and_missing_metrics_are_failures(self):
        before = health()
        for after in [health(uptime=99), health(low=14001), health(heap=0), {}]:
            with self.assertRaises(RuntimeError): smoke.check_health(before, after)
        smoke.check_health(before, health(uptime=101, heap=23500, low=13900))

    def test_cli_business_errors_and_process_errors_are_failures(self):
        for code, text in [(0, '{"error":"denied"}'), (1, '{"ok":true}'), (0, '')]:
            with patch.object(smoke.subprocess, 'run', return_value=subprocess.CompletedProcess(
                    [], code, stdout=text, stderr='')):
                with self.assertRaises(RuntimeError): smoke.cli_json('cli', ['peers'], 1)

    def test_unpaired_or_unadmitted_devices_never_receive_hardware_commands(self):
        for values in [[[]], [[{'nodeId': 'real', 'outbound': True}], {'members': ['other']}]]:
            with patch.object(smoke, 'cli_json', side_effect=values), patch.object(smoke, 'debug_device') as debug:
                with self.assertRaises(RuntimeError): smoke.run_smoke('COM_TEST', 'real', 'cli', count=1)
                debug.assert_not_called()

    def test_sequential_real_device_checks_do_not_claim_ai_chat(self):
        calls = []
        metrics = iter([health(100), health(103), health(106)])
        def cli(_, arguments, timeout):
            calls.append(arguments)
            if arguments == ['peers']: return [{'nodeId': 'real', 'outbound': True}]
            if arguments == ['space', 'show']: return {'members': ['real']}
            if arguments[-1] == 'read': return {'boardId': 'ESP32-2432S028'}
            return health(102 if len(calls) == 4 else 105)
        def debug(port, operation, timeout):
            self.assertEqual((port, operation), ('COM_TEST', 'health'))
            return {'source': 'device-uart', 'result': next(metrics)}
        with patch.object(smoke, 'cli_json', side_effect=cli), patch.object(smoke, 'debug_device', side_effect=debug):
            result = smoke.run_smoke('COM_TEST', 'real', 'cli', count=2, interval=0)
        self.assertTrue(result['ok'])
        self.assertFalse(result['chatEndToEndTested'])
        self.assertEqual(result['authenticatedCalls'], 4)
        self.assertEqual(result['heapFreeDeltaBytes'], 0)
        self.assertEqual(len(calls), 6)
        self.assertTrue(all(call[:2] == ['edge-plugin', 'real'] for call in calls[2:]))

    def test_continuing_heap_growth_after_warmup_is_not_a_pass(self):
        responses = [[{'nodeId': 'real', 'outbound': True}], {'members': ['real']}]
        ticks = iter(range(100, 1000))
        def cli(_, args, timeout):
            if responses: return responses.pop(0)
            if args[-1] == 'read': return {'boardId': 'ESP32-2432S028'}
            return health(next(ticks))
        samples = iter([24000, 24000, 24000, 24000, 24000, 24000, 21000])
        def debug(*args, **kwargs):
            return {'source': 'device-uart', 'result': health(next(ticks), heap=next(samples))}
        with patch.object(smoke, 'cli_json', side_effect=cli), patch.object(smoke, 'debug_device', side_effect=debug):
            with self.assertRaisesRegex(RuntimeError, 'Heap continued shrinking'):
                smoke.run_smoke('COM_TEST', 'real', 'cli', count=6, interval=0)

    def test_wrong_board_is_not_accepted(self):
        with patch.object(smoke, 'cli_json', side_effect=[
                [{'nodeId': 'real', 'outbound': True}], {'members': ['real']},
                {'boardId': 'ESP32-2432S028-SIM'}]), patch.object(smoke, 'debug_device', return_value={
                    'source': 'device-uart', 'result': health()}):
            with self.assertRaises(RuntimeError): smoke.run_smoke('COM_TEST', 'real', 'cli', count=1)


if __name__ == '__main__': unittest.main()
