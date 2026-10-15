import copy
import importlib.util
from pathlib import Path
import sys
import unittest

PERFORMANCE = Path(__file__).parents[1] / 'performance'
sys.path.insert(0, str(PERFORMANCE))
spec = importlib.util.spec_from_file_location(
    'application_benchmark', PERFORMANCE / 'run_compose_application_benchmark.py')
benchmark = importlib.util.module_from_spec(spec)
spec.loader.exec_module(benchmark)


def sample(iteration, frame=40, workload='sidebar-prepared'):
    return {
        'workload': workload, 'iteration': iteration, 'firstObserved': iteration == 0,
        'hostThroughOnLoadObservedMs': frame + 20,
        'businessContentReadyObservedMs': frame + 30,
        'frames': [{'buildMs': 2, 'rasterMs': 3, 'totalSpanMs': 10}],
        'log': f'''event=compose_load_begin package=p route=main context=c{iteration} load=1 updateInputs=false queueMs=0 startedAfterMs=0.1
 event=compose_load_stage package=p route=main context=c{iteration} load=1 stage=read-script stepMs=10 totalMs=10
 event=compose_load_stage package=p route=main context=c{iteration} load=1 stage=acquire-engine stepMs=11 totalMs=11
 event=compose_load_stage package=p route=main context=c{iteration} load=1 stage=render-command stepMs=5 totalMs=16
 event=compose_first_tree_frame package=p route=toolpkg:p:ui:main context=c{iteration} load=1 elapsedMs={frame} frameWaitMs=8 tickerEnabled=true prepared=true
 event=compose_onload_frame package=p context=c{iteration} elapsedMs=12 actionMs=8 frameWaitMs=4''',
    }


class ApplicationBenchmarkTests(unittest.TestCase):
    def report(self):
        result = {'rows': 20, 'repetitions': 3, 'mode': 'profile',
                  'view': {'width': 1000, 'height': 800, 'devicePixelRatio': 2},
                  'environment': {'power': "Now drawing from 'AC Power'"},
                  'samples': [sample(0, 100), sample(1, 20), sample(2, 40)]}
        result['summary'] = benchmark.summarize(result)
        return result

    def test_browser_stages_keep_overlapping_calls_and_polling_delay_separate(self):
        def web_sample(iteration, multiplier):
            marks = {'app-start': 10, 'host-ready': 12, 'content-ready': 50}
            durations = {'host-wait': 2, 'theme': 15, 'current-screen': 5,
                         'snapshot': 30, 'dom-render': 1}
            entries = [{'name': 'character-memory.' + name, 'startTime': value * multiplier,
                        'duration': 0} for name, value in marks.items()]
            entries += [{'name': 'character-memory.' + name, 'startTime': 12 * multiplier,
                         'duration': value * multiplier} for name, value in durations.items()]
            return {'workload': 'main', 'iteration': iteration, 'firstObserved': iteration == 0,
                    'webTiming': {'entries': entries, 'observedAt': 70 * multiplier,
                                  'navigation': [{'responseStart': 2, 'responseEnd': 4}]}}
        result = benchmark.web_content_stages({'samples': [web_sample(0, 10), web_sample(1, 1), web_sample(2, 2)]})
        summary = result['summary']['main']
        self.assertEqual(summary['snapshot']['p50'], 30)
        self.assertEqual(summary['app-to-content']['p50'], 40)
        self.assertEqual(summary['content-observation-delay']['p50'], 20)
        self.assertEqual(summary['document-response']['n'], 2)
        incomplete = web_sample(1, 1)
        incomplete['webTiming']['entries'].pop()
        with self.assertRaisesRegex(ValueError, 'Incomplete browser timing'):
            benchmark.web_content_stages({'samples': [incomplete]})
        self.assertEqual(benchmark.web_content_stages({'samples': [sample(1)]})['samples'], [])
        combined = web_sample(1, 1)
        combined['webTiming']['entries'] = [entry for entry in combined['webTiming']['entries']
                                            if entry['name'] not in ('character-memory.theme', 'character-memory.current-screen', 'character-memory.snapshot')]
        combined['webTiming']['entries'].append({'name': 'character-memory.load-screen', 'startTime': 12, 'duration': 32})
        combined['businessContentReadyObservedMs'] = 120
        durations = benchmark.web_content_stages({'samples': [combined]})['samples'][0]['durationsMs']
        self.assertEqual(durations['load-screen'], 32)
        self.assertEqual(durations['mount-to-dom-estimate'], 100)


    def test_excludes_first_sample_and_uses_within_sample_ratios(self):
        row = self.report()['summary']['sidebar-prepared']
        self.assertEqual(row['firstObserved']['firstFrameMs'], 100)
        self.assertEqual(row['subsequent']['firstFrameMs']['p50'], 20)
        self.assertEqual(row['subsequent']['firstFrameMs']['n'], 2)
        self.assertEqual(row['phaseRatios']['firstFrameWait']['p50'], .2)
        self.assertEqual(row['phaseRatios']['load:read-script']['p50'], 10 / 16)
        self.assertEqual(row['subsequent']['throughOnLoadFrameMs']['p50'], 32)

    def test_rejects_incomplete_or_ambiguous_lifecycle(self):
        value = sample(1)
        value['log'] = value['log'].split('event=compose_onload_frame')[0]
        with self.assertRaisesRegex(ValueError, 'one correlated complete'):
            benchmark.summarize({'samples': [sample(0), value]})
        value = sample(1)
        value['log'] += '\nevent=compose_onload_frame package=p context=missing elapsedMs=1'
        with self.assertRaisesRegex(ValueError, 'one correlated complete'):
            benchmark.summarize({'samples': [sample(0), value]})

    def test_rejects_uncontrolled_power_and_configuration_changes(self):
        baseline = self.report()
        for field, replacement in [('rows', 100), ('mode', 'debug'), ('view', {}), ('repetitions', 5)]:
            current = copy.deepcopy(baseline)
            current[field] = replacement
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, field):
                benchmark.compare(current, baseline, 15)
        current = copy.deepcopy(baseline)
        current['environment']['power'] = "Now drawing from 'Battery Power'"
        with self.assertRaisesRegex(ValueError, 'power'):
            benchmark.compare(current, baseline, 15)
        current = copy.deepcopy(baseline)
        current['environment']['power'] = "Now drawing from 'AC Power'\n -InternalBattery-0 78%; discharging; 16:02 remaining"
        baseline['environment']['power'] = "Now drawing from 'AC Power'\n -InternalBattery-0 80%; discharging; 15:40 remaining"
        self.assertIsNotNone(benchmark.compare(current, baseline, 15))

    def test_paired_paths_are_matched_by_repetition_not_independent_medians(self):
        unprepared = [sample(0, 100, 'sidebar-unprepared'), sample(1, 80, 'sidebar-unprepared'),
                      sample(2, 180, 'sidebar-unprepared')]
        prepared = [sample(0, 100), sample(1, 30), sample(2, 80)]
        result = benchmark.paired_workloads({'samples': prepared + unprepared})
        self.assertEqual([row['iteration'] for row in result['samples']], [1, 2])
        self.assertEqual(result['summary']['hostThroughOnLoadObservedMs']['ratio']['p50'], .5)
        with self.assertRaisesRegex(ValueError, 'Missing same-round'):
            benchmark.paired_workloads({'samples': prepared})

    def test_regression_gate_uses_correlated_page_totals(self):
        baseline = self.report()
        current = copy.deepcopy(baseline)
        current['summary']['sidebar-prepared']['subsequent']['firstFrameMs']['p50'] = 24
        result = benchmark.compare(current, baseline, 15)
        self.assertFalse(result[0]['passed'])
        self.assertTrue(result[1]['passed'])
        self.assertAlmostEqual(result[0]['changePercent'], 20)
        current['summary']['sidebar-prepared']['subsequent']['firstFrameMs']['p50'] = 18
        self.assertTrue(all(row['passed'] for row in benchmark.compare(current, baseline, 15)))

    def test_browser_completion_gates_ignore_polling_and_keep_estimates_explicit(self):
        baseline = self.report()
        baseline['webContentStages'] = {'summary': {'main': {
            'app-to-content': {'p50': 40}, 'mount-to-dom-estimate': {'p50': 160}}}}
        current = copy.deepcopy(baseline)
        current['webContentStages']['summary']['main']['app-to-content']['p50'] = 23
        current['webContentStages']['summary']['main']['mount-to-dom-estimate']['p50'] = 200
        comparison = benchmark.compare(current, baseline, 15)
        browser = {item['metric']: item for item in comparison if item['metric'].startswith('web:')}
        self.assertTrue(browser['web:app-to-content']['passed'])
        self.assertFalse(browser['web:mount-to-dom-estimate']['passed'])
        self.assertEqual(browser['web:mount-to-dom-estimate']['changePercent'], 25)
        baseline.pop('webContentStages')
        self.assertEqual(len(benchmark.compare(current, baseline, 15)), 3)

    def test_pipeline_attribution_sums_per_mount_and_keeps_inclusive_stage_separate(self):
        first = sample(0)
        second = sample(1)
        third = sample(2)
        for item, start in ((first, 1000), (second, 10000), (third, 20000)):
            item.update(startedWallMicros=start, finishedWallMicros=start + 5000)
            item['log'] += f"\nevent=compose_first_tree_frame wallMicros={start + 3000}"
        raw = """event=compose_pipeline_stage stage=js-to-rust durationMs=3 wallMicros=11000 call=a
        event=compose_pipeline_stage stage=js-to-rust durationMs=2 wallMicros=12000 call=a
        event=compose_pipeline_stage stage=js-send-inclusive durationMs=7 wallMicros=12500 call=a
        event=compose_pipeline_stage stage=js-to-rust durationMs=20 wallMicros=14000 call=b
        event=compose_pipeline_stage stage=js-to-rust durationMs=6 wallMicros=21000 call=c
        event=compose_pipeline_stage stage=rust-messagepack durationMs=2 wallMicros=21500 bytes=36000000
        event=compose_pipeline_stage stage=js-to-rust durationMs=99 wallMicros=50000 call=outside"""
        result = benchmark.pipeline_stages({'samples': [first, second, third]}, raw)
        row = result['summary']['sidebar-prepared']
        self.assertEqual(row['beforeFirstFrame']['js-to-rust']['durationMs']['p50'], 5)
        self.assertEqual(row['throughContent']['js-to-rust']['durationMs']['p50'], 6)
        self.assertEqual(row['beforeFirstFrame']['js-send-inclusive']['durationMs']['p50'], 7)
        self.assertEqual(row['beforeFirstFrame']['rust-messagepack']['bytes']['p50'], 36000000)
        self.assertEqual(len(second['pipelineStages']['events']), 4)
        self.assertEqual(len(second['pipelineStages']['beforeFirstFrame']), 3)

    def test_pipeline_coverage_requires_every_page_and_complete_decode_protocol(self):
        common = {'rust-to-js-input', 'js-render-sync', 'js-retained-commit', 'js-send-inclusive',
                  'js-to-rust', 'rust-typed-response', 'rust-core-value', 'rust-messagepack', 'dart-node-apply'}
        for path in ({'dart-stream-messagepack'},
                     {'dart-messagepack', 'dart-watch-read', 'dart-watch-retain', 'dart-watch-reencode'}):
            def page(stages):
                return {**sample(1), 'pipelineStages': {'beforeFirstFrame': [{'stage': s} for s in stages]}}
            complete = page(common | path)
            benchmark.validate_pipeline_capture({'samples': [complete]})
            for missing in common | path:
                with self.subTest(path=path, missing=missing), self.assertRaisesRegex(ValueError, 'Incomplete pipeline'):
                    benchmark.validate_pipeline_capture({'samples': [complete, page((common | path) - {missing})]})

    def test_incomplete_or_corrupt_native_trace_is_rejected(self):
        for raw in ('event=compose_pipeline_stage stage=js-to-rust durationMs=3',
                    'event=compose_pipeline_stage stage=js-to-rust durationMs=3 wallMicros=bad'):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                benchmark.pipeline_stages({'samples': []}, raw)

    def test_diagnostic_tracing_cannot_be_compared_with_untraced_baseline(self):
        baseline = self.report()
        current = copy.deepcopy(baseline)
        current['traceStages'] = True
        with self.assertRaisesRegex(ValueError, 'tracing differs'):
            benchmark.compare(current, baseline, 15)


if __name__ == '__main__':
    unittest.main()
