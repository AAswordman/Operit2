import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    'analyze_compose_load', Path(__file__).parents[1] / 'performance' / 'analyze_compose_load.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ComposeLoadAnalysisTests(unittest.TestCase):
    def test_parallel_stages_use_wall_time_instead_of_sum(self):
        report = module.analyze("""event=compose_load_begin package=p route=main context=c load=1 updateInputs=false queueMs=2 startedAfterMs=20
 event=compose_load_stage package=p route=main context=c load=1 stage=read-script stepMs=10 totalMs=10
 event=compose_load_stage package=p route=main context=c load=1 stage=acquire-engine stepMs=10 totalMs=11
 event=compose_load_stage package=p route=main context=c load=1 stage=render-command stepMs=5 totalMs=16
 event=compose_first_tree_frame package=p route=toolpkg:p:ui:main context=c load=1 elapsedMs=40 frameWaitMs=4 tickerEnabled=true
 event=compose_onload_frame package=p context=c elapsedMs=12 actionMs=8 frameWaitMs=4""")
        sample = report['samples'][0]
        self.assertEqual(sample['loadMs'], 16)
        self.assertEqual(sample['outsideLoadMs'], 24)
        self.assertEqual(sample['throughOnLoadFrameMs'], 52)
        self.assertEqual(sample['onLoadActionMs'], 8)
        self.assertEqual(report['unmatched'], [])

    def test_interleaved_sessions_do_not_mix_onload_times(self):
        report = module.analyze("""event=compose_load_stage package=p route=a stage=open-session stepMs=1 totalMs=1
 event=compose_session_open package=p context=ca session=sa
 event=compose_load_stage package=p route=b stage=open-session stepMs=1 totalMs=1
 event=compose_session_open package=p context=cb session=sb
 event=compose_load_stage package=p route=a stage=render-command stepMs=2 totalMs=3
 event=compose_load_stage package=p route=b stage=render-command stepMs=2 totalMs=3
 event=compose_first_tree_frame package=p route=toolpkg:p:ui:a elapsedMs=10 tickerEnabled=true
 event=compose_first_tree_frame package=p route=toolpkg:p:ui:b elapsedMs=20 tickerEnabled=true
 event=compose_onload_frame package=p context=cb elapsedMs=8
 event=compose_onload_frame package=p context=ca elapsedMs=4""")
        self.assertEqual([s['onLoadMs'] for s in report['samples']], [4, 8])
        self.assertEqual(report['unmatched'], [])

    def test_incomplete_and_hidden_loads_are_retained_without_visible_statistics(self):
        report = module.analyze("""event=compose_load_stage package=p route=main stage=read-script stepMs=1 totalMs=1
 event=compose_first_tree_frame package=p route=main elapsedMs=10 tickerEnabled=false
 event=compose_load_stage package=p route=main stage=read-script stepMs=2 totalMs=2""")
        self.assertEqual(len(report['samples']), 2)
        self.assertEqual(report['summary'], {})
        self.assertIsNone(report['samples'][0]['firstFrameWaitMs'])

    def test_first_observation_is_separated_from_reopens_and_updates(self):
        report = module.analyze("""event=compose_load_stage package=p route=main context=c1 load=1 updateInputs=false stage=render-command stepMs=5 totalMs=5
 event=compose_first_tree_frame package=p route=main context=c1 load=1 elapsedMs=100 tickerEnabled=true
 event=compose_load_stage package=p route=main context=c2 load=1 updateInputs=false stage=render-command stepMs=5 totalMs=5
 event=compose_first_tree_frame package=p route=main context=c2 load=1 elapsedMs=20 tickerEnabled=true
 event=compose_load_begin package=p route=main context=c2 load=2 updateInputs=true queueMs=1 startedAfterMs=20
 event=compose_load_stage package=p route=main context=c2 load=2 updateInputs=true stage=render-command stepMs=5 totalMs=5
 event=compose_onload_frame package=p context=c2 elapsedMs=30""")
        summary = report['summary']['p:main']
        self.assertEqual(summary['observations'], 2)
        self.assertEqual(summary['firstObserved']['firstFrameMs'], 100)
        self.assertEqual(summary['subsequent']['firstFrameMs']['p50'], 20)
        self.assertEqual(report['samples'][1]['onLoadMs'], 30)
        self.assertNotIn('onLoadMs', report['samples'][2])

    def test_nearest_rank_quantiles_and_unknown_values(self):
        self.assertEqual(module.distribution([None, 3, 1, 2])['p95'], 3)
        self.assertIsNone(module.distribution([None]))


if __name__ == '__main__':
    unittest.main()
