#!/usr/bin/env python3
"""Compile profile-guided, serial, local fallthrough traces at build time."""
import argparse
import collections
import json
from pathlib import Path

import generate_guest_aot as aot


def profile_blocks(profile):
    blocks, weights = {}, collections.Counter()
    for site in profile.get('top_sites', []):
        for mapping in site.get('mappings', []):
            instructions = mapping.get('instructions', [])
            if not instructions:
                continue
            pc = int(mapping['block_pc'], 0)
            words = tuple(int(i['raw'], 0) for i in instructions)
            pcs = tuple(int(i['pc'], 0) for i in instructions)
            if any(at != pc + n * 4 for n, at in enumerate(pcs)):
                continue
            # Ambiguous snapshots are not used to infer cross-block edges.
            if pc in blocks and blocks[pc] != words:
                blocks[pc] = None
            elif pc not in blocks:
                blocks[pc] = words
            if site.get('handler') == 'BlockBody':
                weights[pc] += int(site['samples'])
    return blocks, weights


def profile_edges(profile):
    if profile.get('schema') == 'operit.guest-aot-fixture.v1':
        return [(int(t.get('samples', 1)), [int(w, 0) for w in t['source']],
                 [int(w, 0) for w in t['target']]) for t in profile.get('traces', [])]
    if profile.get('schema') != 'operit.execution-diagnostics.v1':
        raise ValueError('unsupported profile')
    blocks, weights = profile_blocks(profile)
    result = []
    for pc, words in blocks.items():
        if not words or not weights[pc]:
            continue
        target = blocks.get(pc + len(words) * 4)
        if target:
            result.append((weights[pc], list(words), list(target)))
    return result


def long_candidates(profile, max_length, max_blocks, folded=False, inline_memory=False, register_memory=False):
    if profile.get('schema') == 'operit.guest-aot-fixture.v1':
        paths = [(int(t.get('samples', 1)), [[int(w, 0) for w in block] for block in t['blocks']])
                 for t in profile.get('paths', [])]
    elif profile.get('schema') == 'operit.execution-diagnostics.v1':
        blocks, weights = profile_blocks(profile)
        paths = []
        for pc, first in blocks.items():
            if not first or not weights[pc]:
                continue
            path, at = [], pc
            while len(path) < max_blocks:
                words = blocks.get(at)
                if not words:
                    break
                path.append(list(words));at += len(words) * 4
            if len(path) >= 3:
                paths.append((weights[pc], path))
    else:
        raise ValueError('unsupported profile')
    result = collections.Counter()
    def supported(word):
        ins = aot.decode(word)
        return ins if ins and (folded or ins[0] != 'address') and (register_memory or ins[0] != 'memory_reg') else None
    for samples, path in paths:
        for count in range(3, min(len(path), max_blocks) + 1):
            blocks = path[:count]
            if any(not block for block in blocks):
                continue
            words, edges = [], []
            valid = True
            for n, block in enumerate(blocks):
                if n == count - 1:
                    prefix = []
                    for word in block:
                        ins = supported(word)
                        if not ins or ins[0].startswith('branch_') or len(prefix) == 8:
                            break
                        prefix.append(word)
                    words.extend(prefix)
                    valid = bool(prefix)
                    break
                last = supported(block[-1])
                conditional = last and last[0] in ('branch_cond', 'branch_compare', 'branch_test')
                if not last or (last[0].startswith('branch_') and not conditional) or (
                        conditional and last[0] == 'branch_cond' and last[1] >= 14):
                    valid = False;break
                body = block[:-1] if conditional else block
                if n == 0:
                    suffix = []
                    for word in reversed(body):
                        ins = supported(word)
                        if not ins or ins[0].startswith('branch_') or len(suffix) == 8:
                            break
                        suffix.insert(0, word)
                    body = suffix
                elif any(not (ins := supported(word)) or ins[0].startswith('branch_') for word in body):
                    valid = False;break
                if not body:
                    valid = False;break
                words.extend(body)
                if conditional:
                    edges.append((len(words), False));words.append(block[-1])
                else:
                    edges.append((len(words), True))
            if valid and len(words) <= max_length and aot.compile_sequence(words, inline_memory,
                                                                           trace_edges=edges) is not None:
                result[(words[0], tuple(edges), *words[1:])] += samples
    return result


def candidates(profile, max_length, folded=False, inline_memory=False, register_memory=False):
    result = collections.Counter()
    for samples, source, target in profile_edges(profile):
        if not source or not target:
            continue
        last = aot.decode(source[-1])
        if not last or (last[0] == 'memory_reg' and not register_memory):
            continue
        conditional = last[0] in ('branch_cond', 'branch_compare', 'branch_test')
        if (last[0].startswith('branch_') and not conditional or
                conditional and last[0] == 'branch_cond' and last[1] >= 14):
            continue
        before = []
        for word in reversed(source[:-1] if conditional else source):
            ins = aot.decode(word)
            if (not ins or ins[0].startswith('branch_') or (ins[0] == 'address' and not folded) or
                    (ins[0] == 'memory_reg' and not register_memory)):
                break
            before.insert(0, word)
            if len(before) >= min(8, max_length - (2 if conditional else 1)):
                break
        after = []
        for word in target:
            ins = aot.decode(word)
            if (not ins or ins[0].startswith('branch_') or (ins[0] == 'address' and not folded) or
                    (ins[0] == 'memory_reg' and not register_memory)):
                break
            after.append(word)
            if len(after) >= min(8, max_length - (2 if conditional else 1)):
                break
        if not before or not after:
            continue
        # Try one longest supported window per observed edge, retaining a
        # bounded candidate set instead of all overlapping substrings.
        accepted = None
        for left in range(len(before), 0, -1):
            for right in range(min(len(after), max_length - left - int(conditional)), 0, -1):
                words = tuple(before[-left:] + ([source[-1]] if conditional else []) + after[:right])
                if aot.compile_sequence(words, inline_memory, trace_index=left,
                                        trace_natural=not conditional) is not None:
                    accepted = (words[0], left | (0 if conditional else 0x10000), *words[1:])
                    break
            if accepted:
                break
        if accepted:
            result[accepted] += samples
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', type=Path, action='append', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--limit', type=int, default=512)
    parser.add_argument('--max-length', type=int, default=32)
    parser.add_argument('--max-blocks', type=int, default=2)
    parser.add_argument('--folded', action='store_true')
    parser.add_argument('--inline-memory', action='store_true')
    parser.add_argument('--register-memory', action='store_true')
    args = parser.parse_args()
    if not 1 <= args.limit <= 65536 or not 3 <= args.max_length <= 64 or not 2 <= args.max_blocks <= 8:
        parser.error('limit must be 1..65536, max-length 3..64 and max-blocks 2..8')
    choices = collections.Counter()
    for profile_path in args.profile:
        profile = json.loads(profile_path.read_text())
        # Guest PCs are meaningful only within one recording. Accumulate the
        # resulting word patterns, never edges between unrelated recordings.
        choices.update(candidates(profile, args.max_length, args.folded,
                                  args.inline_memory, args.register_memory))
        if args.max_blocks > 2:
            choices.update(long_candidates(profile, args.max_length, args.max_blocks, args.folded,
                                           args.inline_memory, args.register_memory))
    if args.max_blocks > 2:
        ranked = sorted(choices, key=lambda pattern: choices[pattern] * (
            len(pattern[1]) if isinstance(pattern[1], tuple) else 1), reverse=True)
        selected = ranked[:args.limit]
    else:
        selected = [pattern for pattern, _ in choices.most_common(args.limit)]
    if not selected:
        raise ValueError('profile contains no supported local fallthrough traces')
    selected.sort(key=lambda item: len(item), reverse=True)
    selected, slots = aot.static_index(selected)
    asm = ['// Build-time translations of guarded local fallthrough traces.']
    cpp = ['#include "cpu/aarch64/guest_aot.h"', '#include "cpu/aarch64/block_graph.h"',
           '#include <algorithm>', '#include <array>', '#include <cstdint>',
           'namespace operit::arm64 {', 'extern "C" {']
    cpp += [f'void operit_guest_trace_{n}();' for n in range(len(selected))]
    cpp += ['}', 'namespace {',
            'struct Trace { const void *entry; uint32_t offset; uint16_t length, blocks; };',
            'struct Bucket { uint32_t key, start, count; };', 'static const uint32_t words[] = {']
    offsets, offset = [], 0
    for item in selected:
        words = (item[0], *item[2:])
        offsets.append(offset)
        cpp.append('    ' + ','.join(f'0x{w:08x}u' for w in words) + ',')
        offset += len(words)
    cpp += ['};', 'static const uint8_t operation_counts[] = {']
    for item in selected:
        words = (item[0], *item[2:])
        cpp.append('    ' + ','.join(str(aot.operation_count(aot.decode(word))) for word in words) + ',')
    cpp += ['};', 'static const uint8_t boundaries[] = {']
    for item in selected:
        words, marker = (item[0], *item[2:]), item[1]
        edges = marker if isinstance(marker, tuple) else ((marker & 0xffff, bool(marker & 0x10000)),)
        boundaries = [0] * len(words)
        for index, natural in edges:
            boundaries[index - int(natural)] = 4 if natural else {
                'branch_cond': 1, 'branch_compare': 2, 'branch_test': 3}[aot.decode(words[index])[0]]
        cpp.append('    ' + ','.join(str(boundary) for boundary in boundaries) + ',')
    cpp += ['};', 'static const Trace traces[] = {']
    for n, item in enumerate(selected):
        words, marker = (item[0], *item[2:]), item[1]
        edges = marker if isinstance(marker, tuple) else ((marker & 0xffff, bool(marker & 0x10000)),)
        compiled = aot.compile_sequence(words, args.inline_memory, f'Lguest_trace_{n}', trace_edges=edges)
        if compiled is None:
            raise ValueError('selected trace cannot be compiled')
        asm += ['.p2align 2', f'.globl SYM(operit_guest_trace_{n})', f'SYM(operit_guest_trace_{n}):',
                '.cfi_startproc', '.cfi_def_cfa x29, 224', '.cfi_offset x29, -224', '.cfi_offset x30, -216',
                '.cfi_offset x19, -208', '.cfi_offset x20, -200', '.cfi_offset x21, -192',
                '.cfi_offset x22, -184', '.cfi_offset x23, -176', '.cfi_offset x24, -168',
                '.cfi_offset x25, -160', '.cfi_offset x26, -152', '.cfi_offset x27, -144', '.cfi_offset x28, -136',
                '.cfi_offset 72, -128', '.cfi_offset 73, -112', '.cfi_offset 74, -96', '.cfi_offset 75, -80',
                '.cfi_offset 76, -64', '.cfi_offset 77, -48', '.cfi_offset 78, -32', '.cfi_offset 79, -16',
                '    DIAG_COUNT ED_EVENT_GUEST_AOT_TRACE',
                *(['    DIAG_COUNT ED_EVENT_GUEST_AOT_LONG_TRACE_ENTRY'] if len(edges) > 1 else []),
                *compiled, '.cfi_endproc']
        cpp.append(f'    {{reinterpret_cast<const void*>(&operit_guest_trace_{n}),{offsets[n]}u,{len(words)}u,{len(edges)+1}u}},')
    cpp += ['};', 'static const Bucket buckets[] = {']
    cpp += [f'    {{0x{s[0]:08x}u,{s[1]}u,{s[2]}u}},' if s else '    {0,0,0},' for s in slots]
    cpp += ['};', 'uint32_t hash_word(uint32_t word) {',
            '    word^=word>>16;word*=0x7feb352du;word^=word>>15;word*=0x846ca68bu;return word^(word>>16);',
            '}', 'const Bucket *find_bucket(uint32_t word) {',
            f'    auto slot=hash_word(word)&{len(slots)-1}u;',
            '    for(;;) {const auto &bucket=buckets[slot];if(!bucket.count)return nullptr;',
            f'        if(bucket.key==word)return &bucket;slot=(slot+1)&{len(slots)-1}u;}}', '}',
            'bool matches(const StaticOperation &op,uint32_t word,uint64_t pc,unsigned branch) {',
            '    const auto info=operation_info(op);',
            ('    if(!info.node || info.node->dead_result || '
             + ('(info.node->constant_result && info.code!=StaticCode::Move)' if args.folded else 'info.node->constant_result')
             + ' || info.node->copy_register!=-1 ||'),
            '       info.pc!=pc || info.node->instruction.raw!=word || info.code==StaticCode::Helper)return false;',
            '    return branch ? (branch==1 ? info.code==StaticCode::Branch :',
            '                     branch==2 ? info.code==StaticCode::CompareBranch : info.code==StaticCode::TestBranch)',
            '                  : !operation_control_edge(info.code);', '}',
            'bool matches_sequence(const std::vector<StaticOperation> &code,size_t at,uint32_t word,uint64_t pc,unsigned branch,unsigned count) {',
            '    if(at+count>=code.size() || !matches(code[at],word,pc,branch))return false;',
            '    if(count==1)return true;',
            '    const auto first=operation_info(code[at]);const auto second=operation_info(code[at+1]);',
            '    return count==2 && first.code==StaticCode::Prepare && first.node==second.node &&',
            '        (second.code==StaticCode::LoadPrepared || second.code==StaticCode::StorePrepared ||',
            '         second.code==StaticCode::Madd || second.code==StaticCode::Msub ||',
            '         second.code==StaticCode::AddPrepared || second.code==StaticCode::SubPrepared ||',
            '         second.code==StaticCode::AndPrepared || second.code==StaticCode::OrPrepared ||',
            '         second.code==StaticCode::XorPrepared);',
            '}', '}',
            'std::size_t guest_trace_block_count(const void *entry) {',
            '    for(const auto &trace:traces)if(trace.entry==entry)return trace.blocks;',
            '    return 0;', '}',
            'bool is_guest_trace_entry(const void *entry) {',
            '    static const auto entries=[] {',
            '        std::array<uintptr_t,sizeof(traces)/sizeof(traces[0])> result{};',
            '        for(size_t i=0;i<result.size();++i)result[i]=reinterpret_cast<uintptr_t>(traces[i].entry);',
            '        std::sort(result.begin(),result.end());return result;',
            '    }();',
            '    const auto address=reinterpret_cast<uintptr_t>(entry);',
            '    return address>=entries.front() && address<=entries.back() &&',
            '        std::binary_search(entries.begin(),entries.end(),address);', '}',
            'std::size_t install_guest_traces(BlockGraph &graph) {',
            '    std::size_t installed=0;',
            '    for(auto &owner:graph.blocks) {auto &code=owner->code;bool done=false;',
            '        for(size_t i=2;i+1<code.size() && !done;++i) {',
            '            const auto first=operation_info(code[i]);if(!first.node)continue;',
            '            if(i>2 && operation_info(code[i-1]).node==first.node)continue;',
            '            const auto *bucket=find_bucket(first.node->instruction.raw);if(!bucket)continue;',
            '            for(uint32_t j=0;j<bucket->count;++j) {const auto &t=traces[bucket->start+j];',
            '                const std::vector<StaticOperation> *current=&code;size_t position=i,n=0;',
            '                for(;n<t.length;++n) {',
            '                    const auto count=operation_counts[t.offset+n],boundary=boundaries[t.offset+n];',
            '                    if(!matches_sequence(*current,position,words[t.offset+n],first.pc+n*4,boundary==4 ? 0:boundary,count))break;',
            '                    position+=count;',
            '                    if(boundary) {',
            '                        if(position!=current->size()-1 || operation_info(current->back()).code!=StaticCode::Next)break;',
            '                        const auto destination=graph.entries.find(first.pc+(n+1)*4);',
            '                        if(destination==graph.entries.end())break;',
            '                        const auto &target=destination->second->code;',
            '                        if(target.size()<3 || current->back().target!=target.data()+1)break;',
            '                        current=&target;position=2;',
            '                    }',
            '                }',
            '                if(n!=t.length || position>=current->size())continue;',
            '                const auto after=operation_info((*current)[position]);',
            '                if(after.code!=StaticCode::Next && after.node==operation_info((*current)[position-1]).node)continue;',
            '                code[i].entry=t.entry;++installed;done=true;break;',
            '            }', '        }', '    } return installed;', '}', '}']
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / 'guest_traces.inc').write_text('\n'.join(asm) + '\n')
    (args.output / 'guest_traces.cpp').write_text('\n'.join(cpp) + '\n')
    print(f'compiled {len(selected)} guest traces from {len(choices)} candidates')


if __name__ == '__main__':
    main()
