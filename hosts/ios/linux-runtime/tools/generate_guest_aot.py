#!/usr/bin/env python3
"""Compile profiled or ELF-sourced AArch64 sequences at build time.

The output is ordinary signed host code. Installation is guarded by the live
graph's original instruction words and lowering metadata; no guest bytes are
made executable. Faulting integer memory operations use the existing checked
memory guard. Conditional and indirect branch tails are optional; unsupported
control flow and relocation-dependent instructions remain on the static executor.
"""
import argparse
import collections
import heapq
import json
import struct
from pathlib import Path


RESIDENT = {0: 19, 1: 20, 2: 21, 19: 22, 20: 23, 30: 3, 31: 24}
SCRATCH = (9, 10, 11, 12, 13, 14, 15)


def decode(word):
    """Return a supported instruction's operation and operands, or None."""
    if not isinstance(word, int) or not 0 <= word <= 0xffffffff:
        return None
    indirect = word & 0xfffffc1f
    if indirect in (0xd61f0000, 0xd63f0000, 0xd65f0000):
        return ('branch_indirect_call' if indirect == 0xd63f0000 else 'branch_indirect',
                (word >> 5) & 31)
    if word & 0xff000010 == 0x54000000:
        return ('branch_cond', word & 15)
    if word & 0x7e000000 == 0x34000000:
        return ('branch_compare', 64 if word >> 31 else 32, word & 31, bool(word & (1 << 24)))
    if word & 0x7e000000 == 0x36000000:
        return ('branch_test', 64 if word >> 31 else 32, word & 31, bool(word & (1 << 24)),
                ((word >> 31) << 5) | ((word >> 19) & 31))
    width = 64 if word >> 31 else 32
    rd, rn, rm = word & 31, (word >> 5) & 31, (word >> 16) & 31
    if (word & 0x7fe00000 == 0x1ac00000 and
            ((word >> 10) & 63) in (2, 3, 8, 9, 10, 11)):
        return ('native_binary', width, rd, rn, rm, word, False)
    if word & 0x3fe00800 == 0x1a800000:
        return ('native_binary', width, rd, rn, rm, word, True)
    if word & 0x7fe00000 == 0x1b000000:
        return ('native_multiply', width, rd, rn, rm, (word >> 10) & 31, word)
    if word & 0x1f000000 == 0x10000000:
        return ('address', 64, rd)
    if word & 0x1f000000 == 0x11000000 and not word & (1 << 23):
        return ('imm', width, rd, rn, (word >> 10) & 4095,
                'sub' if word & (1 << 30) else 'add', 12 if word & (1 << 22) else 0,
                bool(word & (1 << 29)))
    if word & 0x1f200000 == 0x0b000000:
        shift, amount = (word >> 22) & 3, (word >> 10) & 63
        if shift < 3 and amount < width:
            return ('reg', width, rd, rn, rm, 'sub' if word & (1 << 30) else 'add', shift, amount,
                    bool(word & (1 << 29)))
    if word & 0x1f000000 == 0x0a000000:
        opc, invert, shift, amount = (word >> 29) & 3, (word >> 21) & 1, (word >> 22) & 3, (word >> 10) & 63
        if amount < width:
            mnemonic = (('and', 'bic'), ('orr', 'orn'), ('eor', 'eon'), ('and', 'bic'))[opc][invert]
            return ('reg', width, rd, rn, rm, mnemonic, shift, amount, opc == 3)
    if word & 0x1f800000 == 0x12800000:
        opc, shift = (word >> 29) & 3, ((word >> 21) & 3) * 16
        if opc != 1 and shift < width:
            return ('wide', width, rd, (word >> 5) & 65535, ('movn', None, 'movz', 'movk')[opc], shift)
    if word & 0x1f800000 == 0x12000000:
        opc, n, s = (word >> 29) & 3, (word >> 22) & 1, (word >> 10) & 63
        selector = (n << 6) | ((~s) & 63)
        if not (width == 32 and n) and selector >= 2:
            levels = (1 << (selector.bit_length() - 1)) - 1
            if s & levels != levels:
                return ('logical_imm', width, rd, rn, word, opc == 3)
    if word & 0x1f800000 == 0x13000000:
        opc, n, s, r = (word >> 29) & 3, (word >> 22) & 1, (word >> 10) & 63, (word >> 16) & 63
        if opc != 3 and n == (width == 64) and s < width and r < width:
            return ('bitfield', width, rd, rn, word, opc == 1)
    if word & 0x3f000000 == 0x39000000:
        opc, size = (word >> 22) & 3, 1 << (word >> 30)
        if opc in (0, 1):
            return ('memory', size, rd, rn, ((word >> 10) & 4095) * size, opc == 0)
    if word & 0x3f200000 == 0x38000000:
        opc, size, mode = (word >> 22) & 3, 1 << (word >> 30), (word >> 10) & 3
        if opc in (0, 1):
            offset = (word >> 12) & 511
            if offset & 256:
                offset -= 512
            return ('memory_unscaled', size, rd, rn, offset, opc == 0, mode)
    if word & 0x3f200c00 == 0x38200800:
        opc, size, option = (word >> 22) & 3, 1 << (word >> 30), (word >> 13) & 7
        if opc in (0, 1) and option in (2, 3, 6, 7):
            return ('memory_reg', size, rd, rn, rm, option, bool(word & (1 << 12)), opc == 0)
    if word & 0x3a000000 == 0x28000000 and not word & (1 << 26):
        opc, load = word >> 30, bool(word & (1 << 22))
        if opc != 3 and (opc != 1 or load):
            size = 8 if opc == 2 else 4
            offset = (word >> 15) & 127
            if offset & 64:
                offset -= 128
            return ('pair', size, rd, rn, (word >> 10) & 31, offset * size,
                    not load, (word >> 23) & 3, opc == 1)
    return None


def operation_count(instruction):
    if instruction[0] == 'memory_reg':
        return 2
    if instruction[0] == 'reg' and (instruction[7] or instruction[5] in ('bic', 'orn', 'eon')):
        return 2
    if instruction[0] == 'native_multiply':
        return 2 if instruction[5] != 31 or instruction[6] & (1 << 15) else 1
    return 1


def compile_sequence(words, inline_memory=False, label_prefix='Lguest_aot_local', trace_index=None,
                     trace_natural=False, trace_edges=None):
    instructions = [decode(word) for word in words]
    if not instructions or any(i is None for i in instructions):
        return None
    if trace_edges is not None and trace_index is not None:
        return None
    edges = list(trace_edges) if trace_edges is not None else (
        [(trace_index, trace_natural)] if trace_index is not None else [])
    if any(not 1 <= index < len(instructions) or (natural and index == 0)
           for index, natural in edges):
        return None
    if any(a[0] >= b[0] or (not a[1] and a[0] + 1 >= b[0]) for a, b in zip(edges, edges[1:])):
        return None
    conditional_edges = {index for index, natural in edges if not natural}
    if len(instructions) < 2 or any(ins[0].startswith('branch_') and n not in conditional_edges
                                    for n, ins in enumerate(instructions[:-1])):
        return None
    if any(index >= len(instructions) - 1 or instructions[index][0] not in
           ('branch_cond', 'branch_compare', 'branch_test') or
           (instructions[index][0] == 'branch_cond' and instructions[index][1] >= 14)
           for index in conditional_edges):
        return None
    allocated, initialized, dirty = {}, set(), set()
    output, cold = [], []
    cursor = 0
    segment_start = 0
    edge_at = {index: natural for index, natural in edges}
    def edge_fallback(index):
        return f'{label_prefix}_trace_fallback' if len(edges) == 1 else f'{label_prefix}_trace_{index}_fallback'
    slots = [operation_count(ins) for ins in instructions]
    offsets = [sum(slots[:n]) for n in range(len(slots))]

    def reg(number, width, stack=False, read=True):
        if number == 31 and not stack:
            return 'xzr' if width == 64 else 'wzr'
        if number in RESIDENT:
            host = RESIDENT[number]
        else:
            if number not in allocated:
                if len(allocated) == len(SCRATCH):
                    raise ValueError('register capacity')
                allocated[number] = SCRATCH[len(allocated)]
            host = allocated[number]
            if read and number not in initialized:
                output.append(f'    ldr x{host}, [x27, #{304 + number * 8}]')
                initialized.add(number)
        return ('x' if width == 64 else 'w') + str(host)

    def dest(number, width, stack=False):
        value = reg(number, width, stack, read=False)
        if number != 31 or stack:
            initialized.add(number)
            if number not in RESIDENT:
                dirty.add(number)
        return value

    def host_number(name):
        return 31 if name in ('xzr', 'wzr') else int(name[1:])

    def flush():
        for number in sorted(dirty):
            output.append(f'    str x{allocated[number]}, [x27, #{304 + number * 8}]')
        allocated.clear()
        initialized.clear()
        dirty.clear()

    try:
        for sequence_index, ins in enumerate(instructions):
            if edge_at.get(sequence_index, False):
                edge_index = offsets[sequence_index] - offsets[segment_start]
                if edge_index != cursor:
                    output.append(f'    add x28, x28, #{(edge_index - cursor) * 32}')
                fallback = edge_fallback(sequence_index)
                output += [f'    cbz x7, {fallback}',
                           *(['    DIAG_COUNT ED_EVENT_GUEST_AOT_LONG_TRACE_EDGE'] if len(edges) > 1 else []),
                           '    DIAG_COUNT ED_EVENT_GUEST_AOT_TRACE_FALLTHROUGH',
                           '    DIAG_COUNT ED_EVENT_GUEST_AOT_TRACE_NATURAL',
                           '    DIAG_COUNT ED_EVENT_LOCAL_EDGE',
                           '    ldrh w16, [x28, #OP_EDGE_PREFIX]', '    add x26, x6, x16',
                           '    ldr x28, [x28, #OP_TARGET]', '    mov x6, x26',
                           '    DIAG_HANDLER', '    DIAG_COUNT ED_EVENT_BLOCK_ADMITTED',
                           '    add x8, x8, #1', '    add x28, x28, #OP_SIZE']
                cold.append(fallback + ':')
                cold += [f'    str x{allocated[n]}, [x27, #{304 + n * 8}]' for n in sorted(dirty)]
                cold.append('    DISPATCH')
                cursor = 0
                segment_start = sequence_index
            index = offsets[sequence_index] - offsets[segment_start]
            if ins[0].startswith('branch_'):
                kind = ins[0]
                if sequence_index in conditional_edges:
                    if kind != 'branch_cond':
                        _, width, number, nonzero, *extra = ins
                        source = reg(number, width)
                    if index != cursor:
                        output.append(f'    add x28, x28, #{(index - cursor) * 32}')
                    fallback = edge_fallback(sequence_index)
                    output.append(f'    cbz x7, {fallback}')
                    if kind == 'branch_cond':
                        conditions = ('eq', 'ne', 'hs', 'lo', 'mi', 'pl', 'vs', 'vc',
                                      'hi', 'ls', 'ge', 'lt', 'gt', 'le')
                        output += ['    msr nzcv, x25', f'    b.{conditions[ins[1]]} {fallback}']
                    elif kind == 'branch_compare':
                        output.append(f'    {"cbnz" if nonzero else "cbz"} {source}, {fallback}')
                    else:
                        output.append(f'    {"tbnz" if nonzero else "tbz"} {source}, #{extra[0]}, {fallback}')
                    # The installer's target check proves this is a local,
                    # forward BlockBody edge in the currently pinned graph.
                    output += [*(['    DIAG_COUNT ED_EVENT_GUEST_AOT_LONG_TRACE_EDGE'] if len(edges) > 1 else []),
                               '    DIAG_COUNT ED_EVENT_GUEST_AOT_TRACE_FALLTHROUGH',
                               '    DIAG_COUNT ED_EVENT_NEXT_ELIDED', '    DIAG_COUNT ED_EVENT_LOCAL_EDGE',
                               '    DIAG_COUNT ED_EVENT_FALLTHROUGH_LOCAL',
                               '    ldrh w16, [x28, #OP_EDGE_PREFIX]', '    add x26, x6, x16',
                               '    ldr x28, [x28, #OP_SIZE+OP_TARGET]', '    mov x6, x26',
                               '    DIAG_HANDLER', '    DIAG_COUNT ED_EVENT_BLOCK_ADMITTED',
                               '    add x8, x8, #1', '    add x28, x28, #OP_SIZE']
                    cold.append(fallback + ':')
                    cold += [f'    str x{allocated[n]}, [x27, #{304 + n * 8}]' for n in sorted(dirty)]
                    cold.append('    DISPATCH')
                    cursor = 0
                    segment_start = sequence_index + 1
                    continue
                if kind in ('branch_indirect', 'branch_indirect_call'):
                    source = reg(ins[1], 64)
                    output.append(f'    mov x0, {source}')
                    flush()
                    if index != cursor:
                        output.append(f'    add x28, x28, #{(index - cursor) * 32}')
                    output.append('    str x0, [x27, #EF_PENDING_PC]')
                    if kind == 'branch_indirect_call':
                        output.append('    ldr x3, [x28, #OP_IMMEDIATE]')
                    output += ['    DIAG_COUNT ED_EVENT_GUEST_AOT_INDIRECT', '    DIAG_COUNT ED_EVENT_INDIRECT_EDGE',
                               '    ldrh w12, [x28, #OP_EDGE_PREFIX]',
                               '    add x26, x6, x12', '    ldr x16, [x28, #OP_TARGET]',
                               '    cbnz x16, 5f', '    b Lslow', '5:',
                               '    mov x28, x16', '    b LIndirectDispatch']
                    return output + cold
                if kind == 'branch_cond':
                    condition = ins[1]
                    source = None
                else:
                    _, width, number, nonzero, *extra = ins
                    source = reg(number, width)
                flush()
                if index != cursor:
                    output.append(f'    add x28, x28, #{(index - cursor) * 32}')
                if kind == 'branch_cond':
                    if condition < 14:
                        conditions = ('eq', 'ne', 'hs', 'lo', 'mi', 'pl', 'vs', 'vc',
                                      'hi', 'ls', 'ge', 'lt', 'gt', 'le')
                        output += ['    msr nzcv, x25', f'    b.{conditions[condition]} 3f']
                elif kind == 'branch_compare':
                    output.append(f'    {"cbnz" if nonzero else "cbz"} {source}, 3f')
                else:
                    output.append(f'    {"tbnz" if nonzero else "tbz"} {source}, #{extra[0]}, 3f')
                if kind != 'branch_cond' or condition < 14:
                    # Generic edge handling cannot classify local versus cross
                    # graph without another runtime lookup. Leave those
                    # diagnostic counters to the regular branch handlers.
                    output += ['    ldrh w12, [x28, #OP_EDGE_PREFIX]', '    add x26, x6, x12',
                               '    ldr x16, [x28, #OP_SIZE+OP_TARGET]', '    add x28, x28, #OP_SIZE',
                               '    cbnz x16, 5f', '    b Lslow', '5:', '    mov x28, x16', '    DISPATCH']
                output += ['3:', '    ldrh w12, [x28, #OP_EDGE_PREFIX]', '    add x26, x6, x12',
                           '    ldr x16, [x28, #OP_TARGET]', '    cbnz x16, 4f',
                           '    b Lslow', '4:', '    mov x28, x16', '    DISPATCH']
                return output + cold
            kind, width, rd = ins[:3]
            if kind not in ('pair', 'memory_reg', 'memory_unscaled') and (kind != 'memory' or inline_memory):
                operands = {rd}
                if kind in ('imm', 'reg', 'logical_imm', 'bitfield'):
                    operands.add(ins[3])
                if kind == 'memory':
                    operands.add(ins[3])
                if kind in ('reg', 'native_binary', 'native_multiply'):
                    operands.update((ins[3], ins[4]))
                if kind == 'native_multiply':
                    operands.add(ins[5])
                needed = {number for number in operands if number not in RESIDENT}
                if len(set(allocated) | needed) > len(SCRATCH):
                    # Flush only at an instruction boundary, never between
                    # loading its inputs and producing its result.
                    flush()
            if kind == 'address':
                if rd != 31:
                    target = dest(rd, 64)
                    # The original lowering already resolves ADR/ADRP against
                    # the live guest PC; it is not a build-time host address.
                    output.append(f'    ldr {target}, [x28, #{(index - cursor) * 32 + 8}]')
            elif kind == 'imm':
                _, _, _, rn, immediate, mnemonic, shift, sets_flags = ins
                source = reg(rn, width, stack=True)
                target = dest(rd, width, stack=not sets_flags)
                output.append(f'    {mnemonic}{"s" if sets_flags else ""} {target}, {source}, #{immediate}' + (', lsl #12' if shift else ''))
                if sets_flags:
                    output.append('    mrs x25, nzcv')
            elif kind == 'reg':
                _, _, _, rn, rm, mnemonic, shift, amount, sets_flags = ins
                if rd == 31 and not sets_flags:
                    continue
                left, right = reg(rn, width), reg(rm, width)
                target = dest(rd, width)
                suffix = f', {("lsl", "lsr", "asr", "ror")[shift]} #{amount}' if amount else ''
                output.append(f'    {mnemonic}{"s" if sets_flags else ""} {target}, {left}, {right}{suffix}')
                if sets_flags:
                    output.append('    mrs x25, nzcv')
            elif kind == 'native_binary':
                _, _, _, rn, rm, word, reads_flags = ins
                left, right = reg(rn, width), reg(rm, width)
                target = dest(rd, width)
                if reads_flags:
                    output.append('    msr nzcv, x25')
                encoded = ((word & ~((31 << 16) | 0x3ff)) |
                           (host_number(right) << 16) | (host_number(left) << 5) | host_number(target))
                output.append(f'    .long 0x{encoded:08x}')
            elif kind == 'native_multiply':
                _, _, _, rn, rm, ra, word = ins
                left, right, addend = reg(rn, width), reg(rm, width), reg(ra, width)
                target = dest(rd, width)
                encoded = ((word & ~((31 << 16) | (31 << 10) | 0x3ff)) |
                           (host_number(right) << 16) | (host_number(addend) << 10) |
                           (host_number(left) << 5) | host_number(target))
                output.append(f'    .long 0x{encoded:08x}')
            elif kind == 'wide':
                _, _, _, immediate, mnemonic, shift = ins
                if rd == 31:
                    continue
                if mnemonic == 'movk':
                    reg(rd, width)
                target = dest(rd, width)
                output.append(f'    {mnemonic} {target}, #{immediate}' + (f', lsl #{shift}' if shift else ''))
            elif kind in ('logical_imm', 'bitfield'):
                _, _, _, rn, word, *extra = ins
                if kind == 'bitfield' and rd == 31:
                    continue
                source = reg(rn, width)
                if kind == 'bitfield' and extra[0]:
                    reg(rd, width)
                sets_flags = kind == 'logical_imm' and extra[0]
                target = dest(rd, width, stack=kind == 'logical_imm' and not sets_flags)
                encoded = (word & ~0x3ff) | (host_number(source) << 5) | host_number(target)
                output.append(f'    .long 0x{encoded:08x}')
                if sets_flags:
                    output.append('    mrs x25, nzcv')
            elif kind == 'memory_reg':
                _, size, _, rn, rm, option, scaled, writing = ins
                flush()
                # The second lowering record has the retired-instruction prefix.
                # A guard miss resumes after the semantic slow path at record + 1.
                memory_index = index + 1
                if memory_index != cursor:
                    output.append(f'    add x28, x28, #{(memory_index - cursor) * 32}')
                    cursor = memory_index
                output.append(f'    mov x9, x{RESIDENT[rn]}' if rn in RESIDENT else
                              f'    ldr x9, [x27, #{304 + rn * 8}]')
                if rm == 31:
                    output.append('    mov x10, xzr')
                else:
                    output.append(f'    mov x10, x{RESIDENT[rm]}' if rm in RESIDENT else
                                  f'    ldr x10, [x27, #{304 + rm * 8}]')
                extend = {2: 'uxtw', 3: 'lsl', 6: 'sxtw', 7: 'lsl'}[option]
                shift = size.bit_length() - 1 if scaled else 0
                offset = 'w10' if option in (2, 6) else 'x10'
                output.append(f'    add x9, x9, {offset}, {extend} #{shift}')
                output += ['    DIAG_COUNT ED_EVENT_GUEST_AOT_REGISTER_MEMORY_ATTEMPT',
                           '    DIAG_COUNT ' + ('ED_EVENT_MEMORY_WRITE_ATTEMPT' if writing else 'ED_EVENT_MEMORY_READ_ATTEMPT'),
                           f'    bl Lmemoryguard_{int(writing)}_{size}']
                prefix = 'x' if size == 8 else 'w'
                mnemonic = (('strb', 'strh', 'str', 'str') if writing else
                            ('ldrb', 'ldrh', 'ldr', 'ldr'))[size.bit_length() - 1]
                if writing:
                    source = prefix + ('zr' if rd == 31 else str(RESIDENT[rd]) if rd in RESIDENT else '10')
                    if rd != 31 and rd not in RESIDENT:
                        output.append(f'    ldr {source}, [x27, #{304 + rd * 8}]')
                    output.append(f'    {mnemonic} {source}, [x11]')
                else:
                    target = prefix + (str(RESIDENT[rd]) if rd in RESIDENT and rd != 31 else '10')
                    output.append(f'    {mnemonic} {target}, [x11]')
                    if rd != 31 and rd not in RESIDENT:
                        output.append(f'    str x10, [x27, #{304 + rd * 8}]')
                    output += ['    cbz x13, 2f', '1:', '    ldxr w14, [x13]', '    sub w14, w14, #1',
                               '    stlxr w15, w14, [x13]', '    cbnz w15, 1b', '2:']
                output.append('    DIAG_COUNT ED_EVENT_GUEST_AOT_REGISTER_MEMORY_SUCCESS')
            elif kind == 'memory_unscaled':
                _, size, _, rn, offset, writing, mode = ins
                flush()
                if index != cursor:
                    output.append(f'    add x28, x28, #{(index - cursor) * 32}')
                    cursor = index
                output.append(f'    mov x9, x{RESIDENT[rn]}' if rn in RESIDENT else
                              f'    ldr x9, [x27, #{304 + rn * 8}]')
                if mode != 1 and offset:
                    output.append(f'    {"add" if offset > 0 else "sub"} x9, x9, #{abs(offset)}')
                output += ['    DIAG_COUNT ' + ('ED_EVENT_MEMORY_WRITE_ATTEMPT' if writing else 'ED_EVENT_MEMORY_READ_ATTEMPT'),
                           f'    bl Lmemoryguard_{int(writing)}_{size}']
                prefix = 'x' if size == 8 else 'w'
                mnemonic = (('strb', 'strh', 'str', 'str') if writing else
                            ('ldrb', 'ldrh', 'ldr', 'ldr'))[size.bit_length() - 1]
                if writing:
                    source = prefix + ('zr' if rd == 31 else str(RESIDENT[rd]) if rd in RESIDENT else '10')
                    if rd != 31 and rd not in RESIDENT:
                        output.append(f'    ldr {source}, [x27, #{304 + rd * 8}]')
                    output.append(f'    {mnemonic} {source}, [x11]')
                else:
                    target = prefix + (str(RESIDENT[rd]) if rd in RESIDENT and rd != 31 else '10')
                    output.append(f'    {mnemonic} {target}, [x11]')
                    if rd != 31 and rd not in RESIDENT:
                        output.append(f'    str x10, [x27, #{304 + rd * 8}]')
                    output += ['    cbz x13, 2f', '1:', '    ldxr w14, [x13]', '    sub w14, w14, #1',
                               '    stlxr w15, w14, [x13]', '    cbnz w15, 1b', '2:']
                if mode in (1, 3):
                    if mode == 1 and offset:
                        output.append(f'    {"add" if offset > 0 else "sub"} x9, x9, #{abs(offset)}')
                    if rn in RESIDENT:
                        output.append(f'    mov x{RESIDENT[rn]}, x9')
                    else:
                        output.append(f'    str x9, [x27, #{304 + rn * 8}]')
            elif kind == 'pair':
                _, size, _, rn, rt2, offset, writing, mode, signed = ins
                flush()
                if index != cursor:
                    output.append(f'    add x28, x28, #{(index - cursor) * 32}')
                    cursor = index
                if rn in RESIDENT:
                    output.append(f'    mov x9, x{RESIDENT[rn]}')
                else:
                    output.append(f'    ldr x9, [x27, #{304 + rn * 8}]')
                if mode != 1 and offset:
                    output.append(f'    {"add" if offset > 0 else "sub"} x9, x9, #{abs(offset)}')
                output += ['    DIAG_COUNT ' + ('ED_EVENT_MEMORY_WRITE_ATTEMPT' if writing else 'ED_EVENT_MEMORY_READ_ATTEMPT'),
                           f'    bl Lmemoryguard_{int(writing)}_{size * 2}']
                prefix = 'x' if size == 8 or signed else 'w'
                if writing:
                    for target, source in ((0, rd), (1, rt2)):
                        if source == 31:
                            output.append(f'    mov {prefix}{target}, {prefix}zr')
                        elif source in RESIDENT:
                            output.append(f'    mov {prefix}{target}, {prefix}{RESIDENT[source]}')
                        else:
                            output.append(f'    ldr {prefix}{target}, [x27, #{304 + source * 8}]')
                    output.append(f'    stp {prefix}0, {prefix}1, [x11]')
                else:
                    output.append(f'    {"ldpsw" if signed else "ldp"} {prefix}0, {prefix}1, [x11]')
                    for source, target in ((0, rd), (1, rt2)):
                        if target == 31:
                            continue
                        if target in RESIDENT:
                            output.append(f'    mov {prefix}{RESIDENT[target]}, {prefix}{source}')
                        else:
                            output.append(f'    str x{source}, [x27, #{304 + target * 8}]')
                    output += ['    cbz x13, 2f', '1:', '    ldxr w14, [x13]', '    sub w14, w14, #1',
                               '    stlxr w15, w14, [x13]', '    cbnz w15, 1b', '2:']
                if mode in (1, 3):
                    if mode == 1 and offset:
                        output.append(f'    {"add" if offset > 0 else "sub"} x9, x9, #{abs(offset)}')
                    if rn in RESIDENT:
                        output.append(f'    mov x{RESIDENT[rn]}, x9')
                    else:
                        output.append(f'    str x9, [x27, #{304 + rn * 8}]')
            elif inline_memory:
                _, size, _, rn, immediate, writing = ins
                base = reg(rn, 64, stack=True)
                source = reg(rd, 64 if size == 8 else 32) if writing else None
                before_dirty = sorted(dirty)
                prefix = 'x' if size == 8 else 'w'
                target = dest(rd, 64 if size == 8 else 32) if not writing else None
                after_initialized = sorted(initialized)
                miss = f'{label_prefix}_memory_{sequence_index}_miss'
                resume = f'{label_prefix}_memory_{sequence_index}_resume'
                delta = (index - cursor) * 32
                output.append(f'    mov x0, {base}')
                if immediate >= 4096:
                    output.append(f'    add x0, x0, #{immediate >> 12}, lsl #12')
                if immediate & 4095:
                    output.append(f'    add x0, x0, #{immediate & 4095}')
                output += ['    DIAG_COUNT ' + ('ED_EVENT_MEMORY_WRITE_ATTEMPT' if writing else 'ED_EVENT_MEMORY_READ_ATTEMPT'),
                           f'    cbz x7, {miss}', '    eor x1, x0, x0, lsr #13',
                           '    ubfx x1, x1, #12, #13', '    add x1, x7, x1, lsl #5',
                           f'    ldp x16, x17, [x1, #{16 if writing else 0}]',
                           f'    tbnz x16, #0, {miss}', '    sub x16, x0, x16',
                           f'    cmp x16, #{4096 - size}', f'    b.hi {miss}']
                if writing:
                    output += ['    str xzr, [x17, #8]', '    ldr x17, [x1, #8]']
                output += ['    add x1, x17, x0',
                           '    DIAG_COUNT ' + ('ED_EVENT_PAGE_CACHE_WRITE_HIT' if writing else 'ED_EVENT_PAGE_CACHE_READ_HIT')]
                mnemonic = (('strb', 'strh', 'str', 'str') if writing else
                            ('ldrb', 'ldrh', 'ldr', 'ldr'))[size.bit_length() - 1]
                output += [f'    {mnemonic} {source if writing else target}, [x1]', resume + ':']

                # A miss may leave the executor through Lslow. Publish the exact
                # pre-instruction state before calling the existing guard.
                cold.append(miss + ':')
                cold += [f'    str x{allocated[n]}, [x27, #{304 + n * 8}]' for n in before_dirty]
                if delta:
                    cold.append(f'    add x28, x28, #{delta}')
                cold += ['    mov x9, x0', f'    bl Lmemoryguard_{int(writing)}_{size}']
                if writing:
                    if rd == 31:
                        cold_source = prefix + 'zr'
                    elif rd in RESIDENT:
                        cold_source = prefix + str(RESIDENT[rd])
                    else:
                        cold_source = prefix + '0'
                        cold.append(f'    ldr {cold_source}, [x27, #{304 + rd * 8}]')
                    cold.append(f'    {mnemonic} {cold_source}, [x11]')
                else:
                    cold_target = prefix + str(RESIDENT[rd]) if rd in RESIDENT and rd != 31 else prefix + '0'
                    cold.append(f'    {mnemonic} {cold_target}, [x11]')
                    if rd != 31 and rd not in RESIDENT:
                        cold.append(f'    str x0, [x27, #{304 + rd * 8}]')
                    cold += ['    cbz x13, 2f', '1:', '    ldxr w14, [x13]', '    sub w14, w14, #1',
                             '    stlxr w15, w14, [x13]', '    cbnz w15, 1b', '2:']
                cold += [f'    ldr x{allocated[n]}, [x27, #{304 + n * 8}]'
                         for n in after_initialized if n in allocated]
                if delta:
                    cold.append(f'    sub x28, x28, #{delta}')
                cold.append(f'    b {resume}')
            else:
                _, size, _, rn, immediate, writing = ins
                flush()
                if index != cursor:
                    output.append(f'    add x28, x28, #{(index - cursor) * 32}')
                    cursor = index
                if rn in RESIDENT:
                    output.append(f'    mov x9, x{RESIDENT[rn]}')
                else:
                    output.append(f'    ldr x9, [x27, #{304 + rn * 8}]')
                if immediate >= 4096:
                    output.append(f'    add x9, x9, #{immediate >> 12}, lsl #12')
                if immediate & 4095:
                    output.append(f'    add x9, x9, #{immediate & 4095}')
                output += ['    DIAG_COUNT ' + ('ED_EVENT_MEMORY_WRITE_ATTEMPT' if writing else 'ED_EVENT_MEMORY_READ_ATTEMPT'),
                           f'    bl Lmemoryguard_{int(writing)}_{size}']
                prefix = 'x' if size == 8 else 'w'
                if writing:
                    if rd == 31:
                        source = prefix + 'zr'
                    elif rd in RESIDENT:
                        source = prefix + str(RESIDENT[rd])
                    else:
                        source = prefix + '10'
                        output.append(f'    ldr {source}, [x27, #{304 + rd * 8}]')
                    output.append(f'    {("strb", "strh", "str", "str")[size.bit_length()-1]} {source}, [x11]')
                else:
                    target = (prefix + str(RESIDENT[rd])) if rd in RESIDENT and rd != 31 else prefix + '10'
                    output.append(f'    {("ldrb", "ldrh", "ldr", "ldr")[size.bit_length()-1]} {target}, [x11]')
                    if rd != 31 and rd not in RESIDENT:
                        output.append(f'    str x10, [x27, #{304 + rd * 8}]')
                    output += ['    cbz x13, 2f', '1:', '    ldxr w14, [x13]', '    sub w14, w14, #1',
                               '    stlxr w15, w14, [x13]', '    cbnz w15, 1b', '2:']
    except ValueError:
        return None
    flush()
    block_length = sum(slots[segment_start:])
    output += [f'    add x28, x28, #{(block_length - cursor) * 32}', '    DISPATCH']
    return output + cold


def sequences(profile):
    if profile.get('schema') == 'operit.guest-aot-fixture.v1':
        return [(int(block.get('samples', 1)), [int(word, 0) for word in block['words']])
                for block in profile['blocks']]
    if profile.get('schema') != 'operit.execution-diagnostics.v1':
        raise ValueError('unsupported profile')
    return [(int(site['samples']), [int(i['raw'], 0) for i in mapping['instructions']])
            for site in profile.get('top_sites', []) for mapping in site.get('mappings', [])]


def elf_executable_segments(data):
    """Return file-backed executable segments from a bounded AArch64 ELF64 image."""
    if len(data) < 64 or data[:7] != b'\x7fELF\x02\x01\x01':
        raise ValueError('expected a little-endian ELF64 image')
    kind, machine, version = struct.unpack_from('<HHI', data, 16)
    phoff = struct.unpack_from('<Q', data, 32)[0]
    ehsize, entsize, count = struct.unpack_from('<HHH', data, 52)
    if kind not in (2, 3) or machine != 183 or version != 1 or ehsize != 64:
        raise ValueError('expected an AArch64 ET_EXEC or ET_DYN image')
    if entsize != 56 or not 1 <= count <= 1024 or phoff > len(data) or count * entsize > len(data) - phoff:
        raise ValueError('invalid ELF program-header table')
    loads = []
    for i in range(count):
        typ, flags, offset, address, _, size, memory_size, align = struct.unpack_from('<IIQQQQQQ', data, phoff + i * entsize)
        if typ != 1:
            continue
        if (size > memory_size or offset > len(data) or size > len(data) - offset or
                address >= 1 << 48 or memory_size > (1 << 48) - address or
                (align > 1 and (align & (align - 1) or align > 1 << 48 or address % align != offset % align))):
            raise ValueError('invalid ELF load segment')
        if memory_size:
            loads.append((address, memory_size, offset, size, bool(flags & 1)))
    loads.sort()
    if any(a[0] + a[1] > b[0] for a, b in zip(loads, loads[1:])):
        raise ValueError('overlapping ELF load segments')
    segments = []
    for address, _, offset, size, executable in loads:
        if not executable:
            continue
        skip = (-address) & 3
        if size <= skip:
            continue
        size = (size - skip) & ~3
        if size:
            segments.append(memoryview(data)[offset + skip:offset + skip + size])
    if not segments:
        raise ValueError('ELF has no file-backed executable instructions')
    return segments


def elf_sequences(data, include_branches=False, include_addresses=False, include_indirect=False,
                  include_register_memory=False):
    for segment in elf_executable_segments(data):
        run = []
        for (word,) in struct.iter_unpack('<I', segment):
            instruction = decode(word)
            if (instruction is not None and not instruction[0].startswith('branch_') and
                    (include_addresses or instruction[0] != 'address') and
                    (include_register_memory or instruction[0] != 'memory_reg')):
                run.append(word)
            else:
                if (instruction is not None and instruction[0].startswith('branch_') and run and
                        (include_indirect if instruction[0].startswith('branch_indirect') else include_branches)):
                    run.append(word)
                if len(run) >= 2:
                    yield run
                run = []
        if len(run) >= 2:
            yield run


def index_hash(word):
    word ^= word >> 16
    word = (word * 0x7feb352d) & 0xffffffff
    word ^= word >> 15
    word = (word * 0x846ca68b) & 0xffffffff
    return (word ^ (word >> 16)) & 0xffffffff


def static_index(selected):
    groups = {}
    for sequence in selected:
        groups.setdefault(sequence[0], []).append(sequence)
    capacity = 2
    while capacity < 2 * len(groups):
        capacity *= 2
    slots = [None] * capacity
    ordered = []
    for word, group in groups.items():
        slot = index_hash(word) & (capacity - 1)
        while slots[slot] is not None:
            slot = (slot + 1) & (capacity - 1)
        slots[slot] = (word, len(ordered), len(group))
        ordered.extend(group)
    return ordered, slots


def sequence_index(selected):
    """Index full sequences by two words; common prologues need not scan all patterns."""
    groups = {}
    for sequence in selected:
        groups.setdefault(sequence[:2], []).append(sequence)
    capacity = 2
    while capacity < 2 * len(groups):
        capacity *= 2
    slots = [None] * capacity
    ordered = []
    for (first, second), group in groups.items():
        other = index_hash(second)
        slot = (index_hash(first) ^ ((other << 1) | (other >> 31))) & (capacity - 1)
        while slots[slot] is not None:
            slot = (slot + 1) & (capacity - 1)
        slots[slot] = (first, second, len(ordered), len(group))
        ordered.extend(group)
    return ordered, slots


def select_diverse(scores, limit):
    """Defer contained candidates while retaining them as capacity permits.

    Containment is only a ranking heuristic: a shorter pattern can still
    match when its enclosing sequence crosses a block or optimization boundary.
    """
    ordered = scores.most_common()
    selected, deferred, covered = [], [], set()
    for sequence, _ in ordered:
        if sequence in covered:
            deferred.append(sequence)
            continue
        selected.append(sequence)
        for start in range(len(sequence)):
            for end in range(start + 2, len(sequence) + 1):
                covered.add(sequence[start:end])
        if len(selected) == limit:
            return selected
    return (selected + deferred)[:limit]


def select_coverage(scores, occurrences, weights, limit):
    """Greedily cover weighted block edges, retaining zero-gain fallbacks."""
    covered = [0] * len(weights)
    heap = []
    ordered = list(scores.most_common())
    for order, (sequence, _) in enumerate(ordered):
        gain = sum(weights[block] * bin(mask).count('1')
                   for block, mask in occurrences[sequence].items())
        heap.append((-gain, order, sequence))
    heapq.heapify(heap)
    selected = []
    while heap and len(selected) < limit:
        neg_gain, order, sequence = heapq.heappop(heap)
        gain = sum(weights[block] * bin(mask & ~covered[block]).count('1')
                   for block, mask in occurrences[sequence].items())
        if gain != -neg_gain:
            heapq.heappush(heap, (-gain, order, sequence))
            continue
        selected.append(sequence)
        for block, mask in occurrences[sequence].items():
            covered[block] |= mask
    return selected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', type=Path, action='append', default=[],
                        help='repeat for independent training workloads; weights are accumulated')
    parser.add_argument('--elf', type=Path, action='append', default=[])
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--limit', type=int, default=1024)
    parser.add_argument('--max-length', type=int, default=8)
    parser.add_argument('--score-length', action='store_true')
    parser.add_argument('--diverse', action='store_true',
                        help='defer patterns contained in a previously selected pattern')
    parser.add_argument('--coverage', action='store_true',
                        help='rank profile patterns by marginal weighted block-edge coverage')
    parser.add_argument('--branches', action='store_true')
    parser.add_argument('--indirect', action='store_true')
    parser.add_argument('--folded', action='store_true')
    parser.add_argument('--inline-memory', action='store_true')
    parser.add_argument('--register-memory', action='store_true')
    args = parser.parse_args()
    if not args.profile and not args.elf:
        parser.error('at least one profile or ELF is required')
    if args.coverage and (args.elf or args.diverse):
        parser.error('--coverage requires profile-only input and cannot be combined with --diverse')
    if not 1 <= args.limit <= 65536:
        parser.error('limit must be between 1 and 65536')
    if not 2 <= args.max_length <= 64:
        parser.error('max-length must be between 2 and 64')
    candidates = collections.Counter()
    occurrences = {} if args.coverage else None
    # The same block can appear at several sampled handlers and in several
    # workloads. Combine it before compiling its windows: this preserves the
    # old additive weights without recompiling identical windows thousands of times.
    training = collections.Counter()
    for profile_path in args.profile:
        for samples, words in sequences(json.loads(profile_path.read_text())):
            if args.coverage and samples < 0:
                parser.error('--coverage requires nonnegative sample counts')
            training[tuple(words)] += samples
    compilable = {}
    decoded = {}
    def instruction(word):
        if word not in decoded:
            decoded[word] = decode(word)
        return decoded[word]
    for block, (words, samples) in enumerate(training.items()):
        for start in range(len(words)):
            for end in range(start + 2, min(start + args.max_length + 1, len(words) + 1)):
                sequence = tuple(words[start:end])
                if not args.folded and any((ins := instruction(word)) and ins[0] == 'address' for word in sequence):
                    break
                if not args.register_memory and any((ins := instruction(word)) and ins[0] == 'memory_reg' for word in sequence):
                    break
                if (ins := instruction(sequence[-1])) and ins[0].startswith('branch_'):
                    if not (args.indirect if ins[0].startswith('branch_indirect') else args.branches):
                        break
                if sequence not in compilable:
                    compilable[sequence] = compile_sequence(sequence, args.inline_memory) is not None
                if not compilable[sequence]:
                    break
                candidates[sequence] += samples
                if occurrences is not None:
                    blocks = occurrences.setdefault(sequence, {})
                    mask = ((1 << (end - start - 1)) - 1) << start
                    blocks[block] = blocks.get(block, 0) | mask
    for path in args.elf:
        for words in elf_sequences(path.read_bytes(), args.branches, args.folded, args.indirect, args.register_memory):
            for start in range(len(words) - 1):
                for end in range(start + 2, min(start + args.max_length + 1, len(words) + 1)):
                    sequence = tuple(words[start:end])
                    if sequence not in compilable:
                        compilable[sequence] = compile_sequence(sequence, args.inline_memory) is not None
                    if not compilable[sequence]:
                        break
                    candidates[sequence] += 1
    scores = (collections.Counter({sequence: samples * (len(sequence) - 1)
                                   for sequence, samples in candidates.items()})
              if args.score_length else candidates)
    selected = (select_coverage(scores, occurrences, list(training.values()), args.limit)
                if args.coverage else select_diverse(scores, args.limit) if args.diverse else
                [sequence for sequence, _ in scores.most_common(args.limit)])
    if not selected:
        raise ValueError('inputs contain no supported guest sequences')
    selected.sort(key=len, reverse=True)
    selected, slots = sequence_index(selected)
    asm = ['// Build-time translation of verified guest integer sequences.']
    cpp = ['#include "cpu/aarch64/guest_aot.h"', '#include "cpu/aarch64/block_graph.h"',
           '#include <cstdint>', 'namespace operit::arm64 {', 'extern "C" {']
    cpp += [f'void operit_guest_aot_{i}();' for i in range(len(selected))]
    cpp += ['}', 'namespace {', 'struct Pattern { const void *entry; uint32_t offset; uint16_t length; uint8_t branch; };',
            'struct Bucket { uint32_t key, second, start, count; };', 'static const uint32_t words[] = {']
    offsets = []
    offset = 0
    for sequence in selected:
        offsets.append(offset)
        cpp.append('    ' + ','.join(f'0x{word:08x}u' for word in sequence) + ',')
        offset += len(sequence)
    cpp += ['};', 'static const uint8_t operation_counts[] = {']
    for sequence in selected:
        cpp.append('    ' + ','.join(str(operation_count(decode(word))) for word in sequence) + ',')
    cpp += ['};', 'static const Pattern patterns[] = {']
    for n, sequence in enumerate(selected):
        asm += ['.p2align 2', f'.globl SYM(operit_guest_aot_{n})', f'SYM(operit_guest_aot_{n}):',
                '.cfi_startproc', '.cfi_def_cfa x29, 224', '.cfi_offset x29, -224', '.cfi_offset x30, -216',
                '.cfi_offset x19, -208', '.cfi_offset x20, -200', '.cfi_offset x21, -192',
                '.cfi_offset x22, -184', '.cfi_offset x23, -176', '.cfi_offset x24, -168',
                '.cfi_offset x25, -160', '.cfi_offset x26, -152', '.cfi_offset x27, -144', '.cfi_offset x28, -136',
                '.cfi_offset 72, -128', '.cfi_offset 73, -112', '.cfi_offset 74, -96', '.cfi_offset 75, -80',
                '.cfi_offset 76, -64', '.cfi_offset 77, -48', '.cfi_offset 78, -32', '.cfi_offset 79, -16',
                '    DIAG_COUNT ED_EVENT_GUEST_AOT_ENTRY',
                f'    DIAG_COUNT ED_EVENT_GUEST_AOT_SCHEDULED_INSTRUCTIONS, x16, x17, {len(sequence)}']
        compiled = compile_sequence(sequence, args.inline_memory, f'Lguest_aot_{n}')
        if compiled is None:
            raise ValueError('selected sequence cannot be compiled')
        asm += compiled
        asm.append('.cfi_endproc')
        branch = {'branch_cond': 1, 'branch_compare': 2, 'branch_test': 3,
                  'branch_indirect': 4, 'branch_indirect_call': 5}.get(decode(sequence[-1])[0], 0)
        cpp.append(f'    {{reinterpret_cast<const void*>(&operit_guest_aot_{n}),{offsets[n]}u,{len(sequence)}u,{branch}}},')
    cpp += ['};', 'static const Bucket buckets[] = {']
    cpp += [f'    {{0x{slot[0]:08x}u,0x{slot[1]:08x}u,{slot[2]}u,{slot[3]}u}},' if slot else '    {0,0,0,0},' for slot in slots]
    cpp += ['};', 'uint32_t hash_word(uint32_t word) {',
            '    word^=word>>16;word*=0x7feb352du;word^=word>>15;word*=0x846ca68bu;return word^(word>>16);',
            '}',
            'const Bucket *find_bucket(uint32_t word,uint32_t second) {',
            '    const auto other=hash_word(second);',
            f'    auto slot=(hash_word(word)^((other<<1)|(other>>31)))&{len(slots)-1}u;',
            '    for(;;) {const auto &bucket=buckets[slot];if(!bucket.count)return nullptr;',
            f'        if(bucket.key==word && bucket.second==second)return &bucket;slot=(slot+1)&{len(slots)-1}u;}}',
            '}', '}',
            'std::size_t guest_aot_entry_length(const void *entry) {',
            '    for(const auto &p:patterns) if(p.entry==entry)return p.length;',
            '    return 0;', '}',
            'bool is_guest_aot_entry(const void *entry) { return guest_aot_entry_length(entry)!=0; }',
            'std::size_t install_guest_aot(BasicBlock &block) {',
            '    auto &code=block.code;std::size_t installed=0;',
            '    for(size_t i=2;i<code.size();++i) {',
            '        const auto first=operation_info(code[i]);if(!first.node)continue;',
            '        if(i>2 && operation_info(code[i-1]).node==first.node)continue;',
            '        size_t next=i+1;',
            '        if(next<code.size() && operation_info(code[next]).node==first.node)++next;',
            '        if(next>=code.size())continue;',
            '        const auto second=operation_info(code[next]);if(!second.node)continue;',
            '        const auto *bucket=find_bucket(first.node->instruction.raw,second.node->instruction.raw);if(!bucket)continue;',
            '        for(uint32_t j=0;j<bucket->count;++j) {const auto &p=patterns[bucket->start+j];',
            '            if(p.length>code.size()-i-1)continue;',
            '            size_t n=0, operations=0;uint64_t pc=first.pc;',
            '            for(;n<p.length;++n) {',
            '                const auto count=operation_counts[p.offset+n];',
            '                if(i+operations+count>=code.size())break;',
            '                const auto info=operation_info(code[i+operations]);',
            '#if OPERIT_ARM64_GUEST_AOT_TRACES',
            '                if(is_guest_trace_entry(code[i+operations].entry))break;',
            '#endif',
            ('                if(!info.node || info.node->dead_result || '
             + ('(info.node->constant_result && info.code!=StaticCode::Move)' if args.folded else 'info.node->constant_result')
             + ' || info.node->copy_register!=-1 ||'),
            '                   info.pc!=pc+4*n || info.node->instruction.raw!=words[p.offset+n] ||',
            '                   info.code==StaticCode::Helper)break;',
            '                const auto branch=n+1==p.length ? p.branch:0;',
            '                if(count==2) {',
            '                    const auto next=operation_info(code[i+operations+1]);',
            '                    if(info.code!=StaticCode::Prepare || next.node!=info.node ||',
            '                       (next.code!=StaticCode::LoadPrepared && next.code!=StaticCode::StorePrepared &&',
            '                        next.code!=StaticCode::Madd && next.code!=StaticCode::Msub &&',
            '                        next.code!=StaticCode::AddPrepared && next.code!=StaticCode::SubPrepared &&',
            '                        next.code!=StaticCode::AndPrepared && next.code!=StaticCode::OrPrepared &&',
            '                        next.code!=StaticCode::XorPrepared))break;',
            '                }',
            '                if(branch ? (branch==1 ? info.code!=StaticCode::Branch :',
            ('                              branch==2 ? info.code!=StaticCode::CompareBranch :'
             + ('\n                              branch==3 ? info.code!=StaticCode::TestBranch :'
                '\n                              branch==4 ? info.code!=StaticCode::Indirect : info.code!=StaticCode::IndirectCall)'
                if args.indirect else ' info.code!=StaticCode::TestBranch)')),
            '                          : operation_control_edge(info.code))break;',
            '                operations+=count;}',
            '            if(n!=p.length)continue;',
            '            const auto following=operation_info(code[i+operations]);',
            '            if(following.code!=StaticCode::Next && following.node==operation_info(code[i+operations-1]).node)continue;',
            '            code[i].entry=p.entry;i+=operations-1;++installed;break;',
            '        }',
            '    } return installed;', '}', '}']
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / 'guest_aot.inc').write_text('\n'.join(asm) + '\n')
    (args.output / 'guest_aot.cpp').write_text('\n'.join(cpp) + '\n')
    print(f'compiled {len(selected)} guest sequences from {len(candidates)} candidates')


if __name__ == '__main__':
    main()
