#!/usr/bin/env python3
"""Check Worker performance acceptance bookkeeping without running or building a guest."""
import unittest
from profile_workers import performance_summary
from probe_node import validated


def result(seconds, cpus=4, **changes):
    record = {'cpus': cpus, 'wall_seconds': seconds, 'passed': True}
    record.update(changes)
    return record


class WorkerPerformanceTarget(unittest.TestCase):
    def test_every_run_must_meet_target(self):
        summary = performance_summary([result(1), result(2), result(5.01)], 4, 3, 5)
        self.assertTrue(summary['complete'])
        self.assertFalse(summary['passed'])
        self.assertEqual(summary['median_wall_seconds'], 2)
        self.assertEqual(summary['maximum_wall_seconds'], 5.01)

    def test_exact_boundary_and_all_success(self):
        self.assertTrue(performance_summary([result(4), result(5), result(4.9)], 4, 3, 5)['passed'])

    def test_missing_or_duplicate_run_cannot_pass(self):
        for count in (0, 1, 2, 4):
            summary = performance_summary([result(1)] * count, 4, 3, 5)
            self.assertFalse(summary['complete'])
            self.assertFalse(summary['passed'])

    def test_fast_failed_guest_cannot_pass(self):
        rows = [result(1), result(1, passed=False), result(1)]
        self.assertFalse(performance_summary(rows, 4, 3, 5)['passed'])
        # Match the actual original-script validator: quick exit alone is insufficient.
        for changes in ({'stdout': ''}, {'returncode': 1}, {'timed_out': True},
                        {'stdout': '{"test":"workers","ok":true,"workers":4,"increments":39999}'}):
            guest = {'returncode': 0, 'timed_out': False,
                     'stdout': '{"test":"workers","ok":true,"workers":4,"increments":40000}'}
            guest.update(changes)
            self.assertFalse(validated(guest, 'workers'))

    def test_only_selected_cpu_count_is_timed(self):
        rows = [result(7, cpus=1), result(4)] * 3
        self.assertTrue(performance_summary(rows, 4, 3, 5)['passed'])
        self.assertFalse(performance_summary(rows, 1, 3, 5)['passed'])

    def test_invalid_durations_cannot_pass(self):
        for seconds in (-1, float('nan'), float('inf')):
            self.assertFalse(performance_summary([result(seconds)], 4, 1, 5)['passed'])


if __name__ == '__main__':
    unittest.main()
