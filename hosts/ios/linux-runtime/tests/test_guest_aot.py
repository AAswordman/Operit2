#!/usr/bin/env python3
import importlib.util
import collections
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest

path = Path(__file__).resolve().parents[1] / 'tools/generate_guest_aot.py'
spec = importlib.util.spec_from_file_location('guest_aot', path)
aot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(aot)
sys.path.insert(0, str(path.parent))
import generate_guest_traces as traces
import compact_guest_aot_profile as compact_profile


def elf_image(segments):
    """Build a small ELF with (address, flags, payload) load segments."""
    data = bytearray(64 + 56 * len(segments))
    data[:7] = b'\x7fELF\x02\x01\x01'
    struct.pack_into('<HHI', data, 16, 3, 183, 1)
    struct.pack_into('<Q', data, 32, 64)
    struct.pack_into('<HHH', data, 52, 64, 56, len(segments))
    for i, (address, flags, payload) in enumerate(segments):
        offset = len(data)
        struct.pack_into('<IIQQQQQQ', data, 64 + 56 * i,
                         1, flags, offset, address, address, len(payload), len(payload), 1)
        data.extend(payload)
    return data


class GuestAotTests(unittest.TestCase):
    def test_inline_memory_selection_uses_inline_register_requirements(self):
        words = [0xa9065bf5, 0x91050015, 0xa9086bf9, 0xaa0103f9, 0x528000a1, 0xa90973fb,
                 0xaa0303fa, 0xaa0203fb, 0x7943d016, 0x2a0603fc, 0x790152a1, 0xb4000a47]
        self.assertIsNotNone(aot.compile_sequence(words))
        self.assertIsNone(aot.compile_sequence(words, inline_memory=True))
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            profile = directory / 'profile.json'
            profile.write_text(json.dumps({'schema': 'operit.guest-aot-fixture.v1', 'blocks': [
                {'words': [hex(word) for word in words]}]}))
            subprocess.run([sys.executable, str(path), '--profile', str(profile),
                            '--output', str(directory / 'output'), '--max-length', '32',
                            '--branches', '--inline-memory'], check=True, capture_output=True)
            elf = directory / 'input.elf'
            elf.write_bytes(elf_image([(0x10000, 5, struct.pack('<' + 'I'*len(words), *words))]))
            subprocess.run([sys.executable, str(path), '--elf', str(elf),
                            '--output', str(directory / 'elf-output'), '--max-length', '32',
                            '--branches', '--inline-memory', '--folded'], check=True, capture_output=True)

    def test_diverse_selection_defers_contained_sequences(self):
        scores = collections.Counter({(1, 2, 3, 4): 10, (1, 2, 3): 9,
                                      (5, 6): 8, (2, 3): 7})
        self.assertEqual(aot.select_diverse(scores, 2), [(1, 2, 3, 4), (5, 6)])
        self.assertEqual(aot.select_diverse(scores, 4),
                         [(1, 2, 3, 4), (5, 6), (1, 2, 3), (2, 3)])

    def test_coverage_selection_accounts_for_overlap_and_other_blocks(self):
        long, short, independent, repeated = ((1, 2, 3), (1, 2), (4, 5), (6, 6))
        scores = collections.Counter({long: 20, short: 19, independent: 18, repeated: 17})
        occurrences = {long: {0: 0b11}, short: {0: 0b01, 1: 0b1},
                       independent: {2: 0b1}, repeated: {3: 0b11}}
        weights = [10, 8, 9, 1]
        self.assertEqual(aot.select_coverage(scores, occurrences, weights, 4),
                         [long, independent, short, repeated])
        self.assertEqual(aot.select_coverage(scores, occurrences, weights, 2),
                         [long, independent])

    def test_coverage_selection_ties_and_fallback_are_deterministic(self):
        first, second, third = ((1, 2, 3), (1, 2), (2, 3))
        scores = collections.Counter({first: 3, second: 2, third: 1})
        occurrences = {first: {0: 0b11}, second: {0: 0b01}, third: {0: 0b10}}
        for _ in range(2):
            self.assertEqual(aot.select_coverage(scores, occurrences, [1], 5),
                             [first, second, third])

    def test_coverage_cli_generation_and_invalid_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            profile = directory / 'profile.json'
            words = ['0x9100054a', '0x9100056b', '0x9100054a', '0x9100056b']
            profile.write_text(json.dumps({'schema': 'operit.guest-aot-fixture.v1',
                                           'blocks': [{'samples': 2, 'words': words}]}))
            common = [sys.executable, str(path), '--profile', str(profile), '--coverage',
                      '--output', str(directory / 'out')]
            subprocess.run(common, check=True, capture_output=True)
            self.assertTrue((directory / 'out/guest_aot.cpp').exists())
            for extra in (['--diverse'], ['--elf', str(directory / 'unused')]):
                result = subprocess.run(common + extra, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('--coverage requires profile-only input', result.stderr)
            profile.write_text(json.dumps({'schema': 'operit.guest-aot-fixture.v1',
                                           'blocks': [{'samples': -1, 'words': words}]}))
            result = subprocess.run(common, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('nonnegative sample counts', result.stderr)

    def test_compacted_profiles_preserve_generation_and_tie_order(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            inputs = []
            blocks = [
                [{'samples': 2, 'words': ['0x9100054a', '0x9100056b']},
                 {'samples': 1, 'words': ['0x9100058c', '0x910005ad']}],
                [{'samples': 1, 'words': ['0x9100058c', '0x910005ad']},
                 {'samples': 1, 'words': ['0x910005ce', '0x910005ef']}],
            ]
            for index, entries in enumerate(blocks):
                profile = directory / f'{index}.json'
                if index == 0:
                    contents = {'schema': 'operit.execution-diagnostics.v1', 'top_sites': [
                        {'samples': entry['samples'], 'mappings': [{'instructions': [
                            {'raw': word} for word in entry['words']]}]} for entry in entries]}
                else:
                    contents = {'schema': 'operit.guest-aot-fixture.v1', 'blocks': entries}
                profile.write_text(json.dumps(contents))
                inputs.append(profile)
            compacted = directory / 'compact.json'
            result = compact_profile.compact(inputs)
            compacted.write_text(json.dumps(result))
            self.assertEqual([block['samples'] for block in result['blocks']], [2, 2, 1])
            self.assertEqual(result, compact_profile.compact(inputs))
            for input_args, name in ((sum((['--profile', str(p)] for p in inputs), []), 'original'),
                                     (['--profile', str(compacted)], 'compact')):
                subprocess.run([sys.executable, str(path), *input_args, '--output', str(directory / name),
                                '--limit', '1'], check=True, capture_output=True)
            for name in ('guest_aot.cpp', 'guest_aot.inc'):
                self.assertEqual((directory / 'original' / name).read_bytes(),
                                 (directory / 'compact' / name).read_bytes())

    def test_shifted_integer_instructions_count_preparation_records(self):
        for width in (0, 1 << 31):
            for operation in (0x0b000000, 0x4b000000, 0x0a000000,
                              0x2a000000, 0x4a000000, 0x6a000000):
                word = width | operation | (11 << 16) | (6 << 10) | (10 << 5) | 12
                self.assertEqual(aot.operation_count(aot.decode(word)), 2)
                unshifted = word & ~(63 << 10)
                self.assertEqual(aot.operation_count(aot.decode(unshifted)), 1)
                if operation & 0x1f000000 == 0x0a000000:
                    inverted = unshifted | (1 << 21)
                    self.assertEqual(aot.operation_count(aot.decode(inverted)), 2)
        hot = [0x38401661, 0x0b020022, 0x0b022842, 0x4a421842, 0xeb13007f, 0x54ffff61]
        self.assertEqual([aot.operation_count(aot.decode(word)) for word in hot], [1, 1, 2, 2, 1, 1])
        self.assertIsNotNone(aot.compile_sequence(hot, inline_memory=True))

    def test_unscaled_integer_memory_commits_writeback_after_access(self):
        for size in (1, 2, 4, 8):
            for writing in (False, True):
                for mode in range(4):
                    shift = size.bit_length() - 1
                    word = (0x38000000 | shift << 30 | (not writing) << 22 |
                            (504 << 12) | (mode << 10) | (6 << 5) | 11)
                    self.assertEqual(aot.decode(word),
                                     ('memory_unscaled', size, 11, 6, -8, writing, mode))
                    asm = '\n'.join(aot.compile_sequence([word, 0x9100054a]))
                    guard = asm.index(f'bl Lmemoryguard_{int(writing)}_{size}')
                    address = asm.index('sub x9, x9, #8')
                    if mode == 1:
                        self.assertGreater(address, guard)
                    else:
                        self.assertLess(address, guard)
                    if mode in (1, 3):
                        self.assertGreater(asm.index('str x9, [x27, #352]'), guard)
                    else:
                        self.assertNotIn('str x9, [x27, #352]', asm)
        self.assertIsNone(aot.decode(0x3c4084cb))  # vector
        self.assertIsNone(aot.decode(0x388084cb))  # signed load

    def test_two_word_index_separates_common_first_instructions(self):
        patterns = [(1, 2, 3), (1, 2), (1, 9), (4, 9)]
        ordered, slots = aot.sequence_index(patterns)
        self.assertEqual(sorted(ordered), sorted(patterns))
        for first, second, offset, count in (s for s in slots if s):
            self.assertTrue(all(sequence[:2] == (first, second)
                                for sequence in ordered[offset:offset + count]))
        group = next(s for s in slots if s and s[:2] == (1, 2))
        self.assertEqual(ordered[group[2]:group[2] + group[3]], [(1, 2, 3), (1, 2)])

    def test_binary_integer_and_conditional_select_remap_registers(self):
        for width in (0, 1 << 31):
            for operation in (2, 3, 8, 9, 10, 11):
                word = width | 0x1ac00000 | (operation << 10) | (11 << 16) | (10 << 5) | 12
                self.assertEqual(aot.decode(word)[0], 'native_binary')
                generated = '\n'.join(aot.compile_sequence([word, 0x910004a5]))
                expected = width | 0x1ac00000 | (operation << 10) | (10 << 16) | (9 << 5) | 11
                self.assertIn(f'.long 0x{expected:08x}', generated)
            word = width | 0x1a800000 | (11 << 16) | (10 << 5) | 12
            generated = '\n'.join(aot.compile_sequence([word, 0x910004a5]))
            self.assertIn('msr nzcv, x25', generated)
        self.assertIsNone(aot.decode(0x1ac00000 | (4 << 10)))
        self.assertIsNone(aot.decode(0x3a800000))

    def test_multiply_add_sub_remaps_all_operands_and_record_counts(self):
        for width in (0, 1 << 31):
            for sub in (0, 1 << 15):
                word = width | 0x1b000000 | (11 << 16) | sub | (13 << 10) | (10 << 5) | 12
                decoded = aot.decode(word)
                self.assertEqual(decoded, ('native_multiply', 64 if width else 32, 12, 10, 11, 13, word))
                self.assertEqual(aot.operation_count(decoded), 2)
                generated = '\n'.join(aot.compile_sequence([word, 0x910004a5]))
                expected = width | 0x1b000000 | (10 << 16) | sub | (11 << 10) | (9 << 5) | 12
                self.assertIn(f'.long 0x{expected:08x}', generated)
                self.assertIn('add x28, x28, #96', generated)
                zero_addend = (word & ~(31 << 10)) | (31 << 10)
                self.assertEqual(aot.operation_count(aot.decode(zero_addend)), 2 if sub else 1)
                trace = '\n'.join(aot.compile_sequence([word, 0xb400006a, 0x9100056b], trace_index=1))
                self.assertIn('add x28, x28, #64\n    cbz x7, Lguest_aot_local_trace_fallback', trace)
        for reserved in (0x3b0b354c, 0x5b0b354c, 0x1b2b354c):
            result = aot.decode(reserved)
            self.assertTrue(result is None or result[0] != 'native_multiply')

    def test_multiple_profiles_accumulate_weights_before_selection(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            first = directory / 'python.json'
            second = directory / 'node.json'
            first.write_text(json.dumps({'schema': 'operit.guest-aot-fixture.v1', 'blocks': [
                {'samples': 2, 'words': ['0x9100054a', '0x9100056b']},
                {'samples': 2, 'words': ['0x9100054a', '0x9100056b']}]}))
            second.write_text(json.dumps({'schema': 'operit.guest-aot-fixture.v1', 'blocks': [
                {'samples': 2, 'words': ['0x9100054a', '0x9100056b']},
                {'samples': 5, 'words': ['0x9100058c', '0x910005ad']}]}))
            output = directory / 'output'
            subprocess.run([sys.executable, str(path), '--profile', str(first),
                            '--profile', str(second), '--output', str(output), '--limit', '1'],
                           check=True, capture_output=True, text=True)
            generated = (output / 'guest_aot.cpp').read_text()
            self.assertIn('0x9100054au,0x9100056bu', generated)
            self.assertNotIn('0x9100058cu,0x910005adu', generated)

    def test_trace_profiles_do_not_create_edges_between_recordings(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            profiles = []
            # Adjacent guest PCs belong to independent recordings. Neither
            # recording includes a full edge, so no combined trace is valid.
            for n, (pc, words) in enumerate(((0x10000, [0x9100054a, 0x9100056b]),
                                            (0x10008, [0x9100058c, 0x910005ad]))):
                profile = directory / (str(n) + '.json')
                profile.write_text(json.dumps({'schema': 'operit.execution-diagnostics.v1',
                    'top_sites': [{'handler': 'BlockBody', 'samples': 10, 'mappings': [{
                        'block_pc': hex(pc), 'instructions': [
                            {'pc': hex(pc + i * 4), 'raw': hex(word)} for i, word in enumerate(words)]}]}]}))
                profiles.append(profile)
            result = subprocess.run([sys.executable, str(path.parent / 'generate_guest_traces.py'),
                                     '--profile', str(profiles[0]), '--profile', str(profiles[1]),
                                     '--output', str(directory / 'output')],
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('no supported local fallthrough traces', result.stderr)

    def test_register_memory_is_opt_in_and_uses_two_lowering_records(self):
        word = 0xf869d8cb
        self.assertEqual(aot.decode(word), ('memory_reg', 8, 11, 6, 9, 6, True, False))
        asm = '\n'.join(aot.compile_sequence([0x9100054a, word, 0x9100056c]))
        self.assertIn('add x28, x28, #64', asm)
        self.assertIn('add x9, x9, w10, sxtw #3', asm)
        self.assertIn('bl Lmemoryguard_0_8', asm)
        self.assertLess(asm.index('ED_EVENT_GUEST_AOT_REGISTER_MEMORY_ATTEMPT'), asm.index('bl Lmemoryguard_0_8'))
        self.assertGreater(asm.index('ED_EVENT_GUEST_AOT_REGISTER_MEMORY_SUCCESS'), asm.index('bl Lmemoryguard_0_8'))
        trace = '\n'.join(aot.compile_sequence([0x9100054a, word, 0xb400006a, word, 0x9100056c], trace_index=2))
        self.assertIn('add x28, x28, #32\n    cbz x7, Lguest_aot_local_trace_fallback', trace)
        natural = '\n'.join(aot.compile_sequence([word, 0x9100056c], trace_index=1, trace_natural=True))
        self.assertIn('add x28, x28, #32\n    cbz x7, Lguest_aot_local_trace_fallback', natural)
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            profile = directory / 'profile.json'
            profile.write_text(json.dumps({'schema': 'operit.guest-aot-fixture.v1', 'blocks': [
                {'words': ['0x9100054a', hex(word), '0x9100056c']}]}))
            disabled = subprocess.run([sys.executable, str(path), '--profile', str(profile),
                                       '--output', str(directory / 'disabled')], capture_output=True)
            self.assertNotEqual(disabled.returncode, 0)
            output = directory / 'enabled'
            subprocess.run([sys.executable, str(path), '--profile', str(profile), '--output', str(output),
                            '--register-memory'], check=True, capture_output=True)
            generated = (output / 'guest_aot.cpp').read_text()
            generated_asm = (output / 'guest_aot.inc').read_text()
            self.assertIn('1,2,1,', generated)
            self.assertIn('next.node!=info.node', generated)
            self.assertIn('ED_EVENT_GUEST_AOT_SCHEDULED_INSTRUCTIONS, x16, x17, 3', generated_asm)

    def test_register_memory_traces_are_opt_in(self):
        profile = {'schema': 'operit.guest-aot-fixture.v1', 'traces': [
            {'source': ['0x9100054a', '0xf86968cb', '0xb400006a'],
             'target': ['0xf86968ec', '0x9100058d']},
            {'source': ['0xf86968cb'], 'target': ['0x9100056c', '0x9100058d']}]}
        self.assertEqual(len(traces.candidates(profile, 8)), 0)
        self.assertEqual(len(traces.candidates(profile, 8, register_memory=True)), 2)

    def test_elf_executable_segments_and_unaligned_address(self):
        image = elf_image([(0x1001, 5, b'abc' + struct.pack('<II', 0x9100054a, 0x9100096b)),
                           (0x2000, 4, b'data')])
        self.assertEqual([bytes(s) for s in aot.elf_executable_segments(image)],
                         [struct.pack('<II', 0x9100054a, 0x9100096b)])

    def test_elf_segments_never_join_sequences(self):
        word = struct.pack('<I', 0x9100054a)
        image = elf_image([(0x1000, 5, word), (0x2000, 5, word),
                           (0x3000, 5, word * 2 + struct.pack('<I', 0x14000000) + word * 2)])
        self.assertEqual(list(aot.elf_sequences(image)), [[0x9100054a] * 2] * 2)

    def test_elf_branch_tail_does_not_join_next_block(self):
        add, branch = 0x9100054a, 0xb400004a
        image = elf_image([(0x1000, 5, struct.pack('<IIII', add, branch, add, add))])
        self.assertEqual(list(aot.elf_sequences(image)), [[add, add]])
        self.assertEqual(list(aot.elf_sequences(image, True)), [[add, branch], [add, add]])

    def test_elf_rejects_invalid_headers_and_segments(self):
        original = elf_image([(0x1000, 5, b'12345678')])
        cases = []
        for offset, fmt, value in ((16, '<H', 1), (18, '<H', 62), (32, '<Q', len(original) - 10),
                                   (52, '<H', 63), (54, '<H', 55),
                                   (64 + 32, '<Q', 9), (64 + 40, '<Q', 7),
                                   (64 + 48, '<Q', 3)):
            bad = original.copy()
            struct.pack_into(fmt, bad, offset, value)
            cases.append(bad)
        cases.extend((original[:50], original[:-1], elf_image([(0x1000, 4, b'1234')]),
                      elf_image([(0x1000, 5, b'12345678'), (0x1004, 5, b'1234')])))
        for bad in cases:
            with self.subTest(image=bad):
                with self.assertRaises(ValueError):
                    aot.elf_executable_segments(bad)

    def test_static_index_collision_lookup_and_pattern_order(self):
        # Force many collisions, including key zero, then verify that probing
        # and grouping preserve longest-first precedence for every first word.
        words = [w for w in range(10000) if aot.index_hash(w) & 15 == 0][:16]
        selected = [(word, 1, 2, 3) for word in words] + [(word, 9) for word in words]
        ordered, slots = aot.static_index(selected)
        self.assertGreaterEqual(len(slots), 2 * len(words))
        for word in words:
            position = aot.index_hash(word) & (len(slots) - 1)
            while slots[position] is not None and slots[position][0] != word:
                position = (position + 1) & (len(slots) - 1)
            self.assertIsNotNone(slots[position])
            _, start, count = slots[position]
            self.assertEqual(ordered[start:start + count], [(word, 1, 2, 3), (word, 9)])

    def test_reject_control(self):
        for word in (0x14000000, 0xd65f03c1):
            with self.subTest(word=hex(word)):
                self.assertIsNone(aot.decode(word))

    def test_indirect_tail_uses_target_before_call_writes_link(self):
        for word, kind in ((0xd61f0140, 'branch_indirect'), (0xd65f03c0, 'branch_indirect'),
                           (0xd63f03c0, 'branch_indirect_call')):
            with self.subTest(word=hex(word)):
                self.assertEqual(aot.decode(word)[0], kind)
                code = '\n'.join(aot.compile_sequence([0x9100054a, word]))
                self.assertIn('str x0, [x27, #EF_PENDING_PC]', code)
                self.assertIn('b LIndirectDispatch', code)
                self.assertLess(code.index('str x0, [x27, #EF_PENDING_PC]'), code.index('b LIndirectDispatch'))
                if kind == 'branch_indirect_call':
                    self.assertIn('mov x0, x3', code)
                    self.assertLess(code.index('mov x0, x3'), code.index('ldr x3, [x28, #OP_IMMEDIATE]'))
                self.assertIsNone(aot.compile_sequence([0x9100054a, word, 0x9100054a]))

    def test_indirect_candidates_are_independently_opted_in(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            profile = directory / 'profile.json'
            profile.write_text(json.dumps({'schema': 'operit.guest-aot-fixture.v1', 'blocks': [
                {'words': ['0xf9400400', '0xd65f03c0']}]}))
            for flags, enabled in (([], False), (['--branches'], False), (['--indirect'], True)):
                output = directory / ('output' + str(len(list(directory.iterdir()))))
                result = subprocess.run([sys.executable, str(path), '--profile', str(profile), '--output', str(output),
                                         *flags], capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, enabled, result.stderr)
                if enabled:
                    self.assertIn('0xd65f03c0u', (output / 'guest_aot.cpp').read_text())
            image = elf_image([(0x1000, 5, struct.pack('<II', 0xf9400400, 0xd65f03c0))])
            self.assertEqual(list(aot.elf_sequences(image)), [])
            self.assertEqual(list(aot.elf_sequences(image, include_indirect=True)), [[0xf9400400, 0xd65f03c0]])

    def test_trace_preserves_temporaries_and_restarts_at_destination(self):
        words = [0x9100054a, 0xb400006a, 0xf94000cb, 0x8b0a016c]
        result = '\n'.join(aot.compile_sequence(words, True, 'Ltrace', 1))
        hot, cold = result.split('Ltrace_trace_fallback:')
        self.assertIn('cbz x7, Ltrace_trace_fallback', hot)
        self.assertIn('cbz x9, Ltrace_trace_fallback', hot)
        self.assertIn('ldr x28, [x28, #OP_SIZE+OP_TARGET]', hot)
        self.assertIn('mov x6, x26', hot)
        self.assertIn('add x12, x11, x9', hot)
        self.assertIn('add x28, x28, #64\n    DISPATCH', hot)
        self.assertIn('str x9, [x27, #384]\n    DISPATCH', cold)
        self.assertIn('Ltrace_memory_2_miss:', cold)
        self.assertNotIn('add x28, x28, #32\n    mov x9, x0', cold)

    def test_trace_candidates_and_generation(self):
        fixture = {'schema': 'operit.guest-aot-fixture.v1', 'traces': [
            {'source': ['0x9100054a', '0xb400006a'],
             'target': ['0xf94000cb', '0x8b0a016c']}]}
        self.assertEqual(len(traces.candidates(fixture, 8)), 1)
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            profile = directory / 'profile.json'
            profile.write_text(json.dumps(fixture))
            subprocess.run([sys.executable, str(path.parent / 'generate_guest_traces.py'),
                            '--profile', str(profile), '--output', str(directory), '--inline-memory'],
                           check=True, capture_output=True, text=True)
            self.assertIn('ED_EVENT_GUEST_AOT_TRACE_FALLTHROUGH', (directory / 'guest_traces.inc').read_text())
            self.assertIn('current->back().target!=target.data()+1', (directory / 'guest_traces.cpp').read_text())
            self.assertIn('std::sort(result.begin(),result.end())', (directory / 'guest_traces.cpp').read_text())

    def test_multiple_block_boundaries(self):
        add, branch, load = 0x9100054a, 0xb400006a, 0xf94000cb
        words = [add, branch, load, add, add, load]
        edges = [(1, False), (4, True)]
        asm = '\n'.join(aot.compile_sequence(words, False, 'Llong', trace_edges=edges))
        self.assertIn('cbz x7, Llong_trace_1_fallback', asm)
        self.assertIn('cbz x7, Llong_trace_4_fallback', asm)
        self.assertEqual(asm.count('ED_EVENT_BLOCK_ADMITTED'), 2)
        self.assertIn('ldr x28, [x28, #OP_SIZE+OP_TARGET]', asm)
        self.assertIn('ldr x28, [x28, #OP_TARGET]', asm)
        self.assertIsNone(aot.compile_sequence(words, trace_edges=[(4, True), (1, False)]))
        self.assertIsNone(aot.compile_sequence(words, trace_edges=[(1, False), (2, True)]))
        profile = {'schema': 'operit.guest-aot-fixture.v1', 'paths': [
            {'blocks': [[hex(add), hex(branch)], [hex(load), hex(add)], [hex(add), hex(load)]]}]}
        choices = traces.long_candidates(profile, 16, 3)
        self.assertEqual(len(choices), 1)
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            source = directory / 'profile.json'
            source.write_text(json.dumps(profile))
            subprocess.run([sys.executable, str(path.parent / 'generate_guest_traces.py'),
                            '--profile', str(source), '--output', str(directory),
                            '--max-blocks', '3', '--max-length', '16'], check=True, capture_output=True)
            generated = (directory / 'guest_traces.cpp').read_text()
            self.assertIn('static const uint8_t boundaries[]', generated)
            self.assertIn('0,2,0,4,0,0,', generated)
            self.assertIn('current=&target;position=2', generated)

    def test_natural_trace_crosses_single_instruction_block(self):
        words = [0x9100054a, 0xf94000cb, 0x9100056c]
        code = '\n'.join(aot.compile_sequence(words, True, 'Lnatural', 1, True))
        hot, cold = code.split('Lnatural_trace_fallback:')
        self.assertIn('add x28, x28, #32\n    cbz x7, Lnatural_trace_fallback', hot)
        self.assertIn('ldr x28, [x28, #OP_TARGET]', hot)
        self.assertIn('str x9, [x27, #384]', cold)
        fixture = {'schema': 'operit.guest-aot-fixture.v1', 'traces': [
            {'source': ['0x9100054a'], 'target': ['0xf94000cb', '0x9100056c']}]}
        self.assertEqual(len(traces.candidates(fixture, 8)), 1)

    def test_conditional_branches_only_end_sequences(self):
        for word, expected in ((0x54000040, 'b.eq 3f'), (0xb400004a, 'cbz x9, 3f'),
                               (0x3700004a, 'tbnz w9, #0, 3f')):
            with self.subTest(word=hex(word)):
                code = '\n'.join(aot.compile_sequence([0x9100054a, word]))
                self.assertIn(expected, code)
                self.assertIn('    ldr x16, [x28, #OP_SIZE+OP_TARGET]', code)
                self.assertIsNone(aot.compile_sequence([0x9100054a, word, 0x9100054a]))
        for cond in (14, 15):
            code = '\n'.join(aot.compile_sequence([0x9100054a, 0x54000040 | cond]))
            self.assertNotIn('OP_SIZE+OP_TARGET', code)

    def test_branch_candidates_require_explicit_opt_in(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            profile = directory / 'profile.json'
            profile.write_text(json.dumps({'schema': 'operit.guest-aot-fixture.v1', 'blocks': [
                {'words': ['0x9100054a', '0x9100056b', '0xb400004a']}]}))
            counts = []
            for flags in ([], ['--branches']):
                output = directory / ('enabled' if flags else 'disabled')
                subprocess.run([sys.executable, str(path), '--profile', str(profile), '--output', str(output),
                                *flags], check=True, capture_output=True, text=True)
                counts.append((output / 'guest_aot.inc').read_text().count('.globl SYM(operit_guest_aot_'))
            self.assertEqual(counts, [1, 3])

    def test_folded_windows_require_explicit_opt_in(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            profile = directory / 'profile.json'
            profile.write_text(json.dumps({'schema': 'operit.guest-aot-fixture.v1', 'blocks': [
                {'words': ['0x9100054a', '0x8b09014b']}]}))
            outputs = []
            for flags in ([], ['--folded']):
                output = directory / ('enabled' if flags else 'disabled')
                subprocess.run([sys.executable, str(path), '--profile', str(profile), '--output', str(output),
                                *flags], check=True, capture_output=True, text=True)
                outputs.append((output / 'guest_aot.cpp').read_text())
            self.assertIn('info.node->dead_result || info.node->constant_result ||', outputs[0])
            self.assertIn('(info.node->constant_result && info.code!=StaticCode::Move)', outputs[1])

    def test_pc_relative_address_uses_live_lowered_immediate(self):
        for adr in (0x1000004a, 0xb000000a):
            result = '\n'.join(aot.compile_sequence([adr, 0x9100054b]))
            self.assertIn('ldr x9, [x28, #8]', result)
            self.assertIn('add x10, x9, #1', result)
        result = '\n'.join(aot.compile_sequence([0xf94000cb, 0x1000004a, 0x9100054b]))
        self.assertIn('ldr x9, [x28, #40]', result)
        self.assertNotIn('ldr xzr', '\n'.join(aot.compile_sequence([0x1000005f, 0x9100054b])))

    def test_pc_relative_candidates_require_folded_opt_in(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            profile = directory / 'profile.json'
            profile.write_text(json.dumps({'schema': 'operit.guest-aot-fixture.v1', 'blocks': [
                {'words': ['0x1000004a', '0x9100054b', '0x8b09014b']}]}))
            outputs = []
            for flags in ([], ['--folded']):
                output = directory / ('enabled' if flags else 'disabled')
                subprocess.run([sys.executable, str(path), '--profile', str(profile), '--output', str(output),
                                *flags], check=True, capture_output=True, text=True)
                outputs.append((output / 'guest_aot.cpp').read_text())
            self.assertNotIn('0x1000004au', outputs[0])
            self.assertIn('0x1000004au', outputs[1])

    def test_flags_are_committed_and_zero_destination_is_not_stack(self):
        # adds x10,x10,#1; subs xzr,x10,x9; ands w11,w10,#0xff
        result = '\n'.join(aot.compile_sequence([0xb100054a, 0xeb09015f, 0x72001d4b]))
        self.assertIn('adds x9, x9, #1\n    mrs x25, nzcv', result)
        self.assertIn('subs xzr, x9, x10\n    mrs x25, nzcv', result)
        self.assertIn('.long 0x72001d2b\n    mrs x25, nzcv', result)
        self.assertNotIn('str x24', result)

    def test_registers_stay_live_and_flush_at_exit(self):
        result = '\n'.join(aot.compile_sequence([0x9100054a, 0x8b09014b, 0xca0d016c]))
        self.assertEqual(result.count('ldr x9, [x27, #384]'), 1)
        self.assertEqual(result.count('str x9, [x27, #384]'), 1)
        self.assertIn('add x28, x28, #96\n    DISPATCH', result)

    def test_write_before_read_avoids_stale_load(self):
        # movz x10,#1; add x11,x10,#2
        result = '\n'.join(aot.compile_sequence([0xd280002a, 0x9100094b]))
        self.assertNotIn('ldr x9, [x27, #384]', result)
        self.assertIn('movz x9, #1', result)
        self.assertIn('str x9, [x27, #384]', result)

    def test_word_register_sp_and_zero(self):
        # add w10,w10,#1; add wsp,wsp,#16; orr w11,wzr,w10
        result = '\n'.join(aot.compile_sequence([0x1100054a, 0x110043ff, 0x2a0a03eb]))
        self.assertIn('add w9, w9, #1', result)
        self.assertIn('add w24, w24, #16', result)
        self.assertIn('orr w10, wzr, w9', result)
        self.assertIn('str x9, [x27, #384]', result)

    def test_movk_reads_old_value_and_shifted_logical(self):
        # movk x10,#0x1234,lsl #16; eor x11,x10,x9,ror #5
        result = '\n'.join(aot.compile_sequence([0xf2a2468a, 0xcac9154b]))
        self.assertIn('ldr x9, [x27, #384]', result)
        self.assertIn('movk x9, #4660, lsl #16', result)
        self.assertIn('ror #5', result)

    def test_scratch_exhaustion_flushes_at_instruction_boundary(self):
        words = [0x91000400 | (n << 5) | n for n in range(9, 17)]
        result = '\n'.join(aot.compile_sequence(words))
        self.assertIn('str x15, [x27, #424]\n    ldr x9, [x27, #432]', result)
        self.assertIn('str x9, [x27, #432]', result)

    def test_memory_flushes_before_guard_and_restores_cursor(self):
        result = '\n'.join(aot.compile_sequence([0x9100054a, 0xf94000cb, 0x8b0a016c, 0xf90004cc]))
        self.assertIn('str x9, [x27, #384]\n    add x28, x28, #32', result)
        self.assertIn('bl Lmemoryguard_0_8', result)
        self.assertIn('bl Lmemoryguard_1_8', result)
        self.assertIn('cbz x13, 2f', result)
        self.assertTrue(result.endswith('add x28, x28, #32\n    DISPATCH'))

    def test_inline_memory_keeps_temporaries_and_publishes_cold_state(self):
        words = [0x9100054a, 0xf94000cb, 0x8b0a016c, 0xf90004cc]
        result = '\n'.join(aot.compile_sequence(words, True, 'Ltest'))
        hot, cold = result.split('Ltest_memory_1_miss:')
        self.assertIn('ldr x11, [x1]\nLtest_memory_1_resume:', hot)
        self.assertIn('add x12, x11, x9', hot)
        self.assertIn('str x12, [x1]', hot)
        self.assertNotIn('bl Lmemoryguard', hot)
        self.assertIn('str x9, [x27, #384]\n    add x28, x28, #32\n    mov x9, x0', cold)
        self.assertIn('ldr x11, [x27, #392]', cold)
        self.assertIn('sub x28, x28, #32\n    b Ltest_memory_1_resume', cold)
        self.assertIn('Ltest_memory_3_miss:', cold)

    def test_inline_memory_alias_and_branch_emit_cold_paths(self):
        # add x10,x10,#1; ldr w10,[x10]; cbz w10,...
        result = '\n'.join(aot.compile_sequence([0x9100054a, 0xb940014a, 0x3400004a], True, 'Lalias'))
        self.assertIn('ldr w9, [x1]', result)
        self.assertIn('cbz w9, 3f', result)
        self.assertIn('Lalias_memory_1_miss:', result)
        self.assertIn('str x9, [x27, #384]', result)
        self.assertIn('ldr x9, [x27, #384]', result)

    def test_unsupported_signed_and_vector_loads(self):
        self.assertIsNone(aot.decode(0xb98000cb))  # ldrsw
        self.assertIsNone(aot.decode(0x3dc000c0))  # ldr q0

    def test_pair_checks_whole_access_and_commits_writeback_last(self):
        result = '\n'.join(aot.compile_sequence([0xa9c130cb, 0xa88130cb]))
        self.assertIn('add x9, x9, #16', result)
        self.assertIn('bl Lmemoryguard_0_16', result)
        self.assertIn('ldp x0, x1, [x11]', result)
        self.assertIn('bl Lmemoryguard_1_16', result)
        self.assertIn('stp x0, x1, [x11]', result)
        self.assertIn('add x9, x9, #16\n    str x9, [x27, #352]', result)
        self.assertIsNone(aot.decode(0x2d400000))  # vector pair
        self.assertIsNone(aot.decode(0x69000000))  # reserved signed pair store

    def test_logical_immediate_and_bitfield_register_rewrite(self):
        # and x10,x11,#0xff; ubfm x11,x10,#8,#63; bfm x12,x11,#0,#7
        result = '\n'.join(aot.compile_sequence([0x92401d6a, 0xd348fd4b, 0xb3401d6c]))
        self.assertIn('.long 0x92401d2a', result)
        self.assertIn('.long 0xd348fd49', result)
        self.assertIn('ldr x11, [x27, #400]', result)
        self.assertIn('.long 0xb3401d2b', result)

    def test_invalid_bit_masks_and_out_of_range_words(self):
        for word in (-1, 0x100000000, 0x12400000, 0x1200fc00,
                     0x13400000, 0xd3000000, 0x73000000):
            with self.subTest(word=hex(word)):
                self.assertIsNone(aot.decode(word))

    def test_logical_immediate_destination_31_is_stack(self):
        result = '\n'.join(aot.compile_sequence([0x12001d5f, 0x110007eb]))
        self.assertIn('.long 0x12001d38', result)
        self.assertIn('add w10, w24, #1', result)


if __name__ == '__main__':
    unittest.main()
