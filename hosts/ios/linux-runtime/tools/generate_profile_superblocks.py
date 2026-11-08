#!/usr/bin/env python3
"""Compose profile-selected static handlers into build-time native superblocks.

This is an experimental compiler: input is diagnostic JSON from an Operit
build with the same static executor ABI. It never runs in the guest process.
"""
import argparse
import collections
import json
from pathlib import Path
import re


def dead_scratch(lines):
    """Whether x12 is overwritten before any subsequent read or branch."""
    for line in lines:
        text = line.strip()
        if re.match(r'(?:b(?:\.[a-z]+|l|r)?|cbz|cbnz|tbz|tbnz)\s', text):
            return False
        if re.search(r'\b[wx]12\b', text):
            match = re.match(r'(?:ldr|ldrh|ldrb|mov)\s+[wx]12,\s*(.*)', text)
            return bool(match and not re.search(r'\b[wx]12\b', match.group(1)))
    return True


def specialize(body, op):
    """Use constants only for fields checked by the resulting install guard."""
    lines = body.splitlines()
    out, guards = [], {}
    i = 0
    while i < len(lines):
        line = lines[i]
        field = re.fullmatch(r'    ldrh w12, \[x28, #OP_([ABD])\]', line)
        if field:
            name = field.group(1).lower()
            value = op[name]
            if not isinstance(value, int) or not 0 <= value <= 65535:
                raise ValueError(f'invalid operation {name}: {value!r}')
            guards[name] = value
            if i + 1 < len(lines):
                access = re.fullmatch(r'    (ldr|str) ([xwq]\d+), \[x27, x12\]', lines[i + 1])
                if access:
                    width = {'x': 8, 'w': 4, 'q': 16}[access.group(2)[0]]
                    stores_scratch = access.group(1) == 'str' and access.group(2) in {'x12', 'w12'}
                    if not stores_scratch and value % width == 0 and value // width < 4096 and dead_scratch(lines[i + 2:]):
                        out.append(f'    {access.group(1)} {access.group(2)}, [x27, #{value}]')
                        i += 2
                        continue
            out.append(f'    mov w12, #{value}')
        else:
            immediate = re.fullmatch(r'    ldr (x\d+), \[x28, #OP_IMMEDIATE\]', line)
            value = int(op['immediate'], 0) if immediate else 0
            if immediate and 0 <= value <= 65535:
                guards['immediate'] = value
                if immediate.group(1) == 'x12' and value <= 4095 and i + 1 < len(lines):
                    add = re.fullmatch(r'    add (x\d+), (x\d+), x12', lines[i + 1])
                    if add and add.group(1) != 'x12' and add.group(2) != 'x12' and dead_scratch(lines[i + 2:]):
                        out.append(f'    add {add.group(1)}, {add.group(2)}, #{value}')
                        i += 2
                        continue
                out.append(f'    mov {immediate.group(1)}, #{value}')
            else:
                out.append(line)
        i += 1
    return '\n'.join(out), tuple(sorted(guards.items()))


def promote_shadow(bodies, names):
    """Keep one repeatedly accessed shadow register in x14 over pure bodies."""
    pure = {'Move', 'Keep', 'Add', 'Sub', 'Adc', 'Sbc', 'And', 'Or', 'Xor',
            'Lsl', 'Lsr', 'Asr', 'Ror', 'Mul', 'Udiv', 'Sdiv', 'AddPrepared',
            'SubPrepared', 'AndPrepared', 'OrPrepared', 'XorPrepared', 'Bitfield', 'Unary'}
    if any(name not in pure for name in names):
        return bodies, False
    full = '\n'.join(bodies)
    if re.search(r'\b[wx]14\b|^\s*(?:b(?:\.[a-z]+|l|r)?|cbz|cbnz|tbz|tbnz)\s', full, re.M):
        return bodies, False
    if re.search(r'\[x27,\s*x\d+\]', full):
        return bodies, False
    access = re.compile(r'(?m)^    (ldr|str) ([xw]\d+), \[x27, #(\d+)\]$')
    # Delayed stores are safe only when every frame access is recognized.
    if any('x27' in line and not access.fullmatch(line) for line in full.splitlines()):
        return bodies, False
    matches = list(access.finditer(full))
    counts = collections.Counter(int(m.group(3)) for m in matches if m.group(2).startswith('x') or m.group(1) == 'ldr')
    if not counts:
        return bodies, False
    offset, count = counts.most_common(1)[0]
    if count < 2 or offset < 304 or offset >= 560 or offset % 8:
        return bodies, False
    if any(int(m.group(3)) != offset and int(m.group(3)) < offset + 8 and
           int(m.group(3)) + (8 if m.group(2).startswith('x') else 4) > offset for m in matches):
        return bodies, False
    # No other direct use of the selected shadow slot may escape substitution.
    if sum(int(m.group(3)) == offset for m in matches) != count:
        return bodies, False
    valid = False
    dirty = False
    result = []
    for body in bodies:
        def replace(match):
            nonlocal valid, dirty
            kind, reg, value = match.groups()
            if int(value) != offset:
                return match.group(0)
            if kind == 'str':
                if not reg.startswith('x'):
                    raise ValueError('partial shadow store cannot be promoted')
                valid = dirty = True
                return f'    mov x14, {reg}'
            prefix = '' if valid else f'    ldr x14, [x27, #{offset}]\n'
            valid = True
            return prefix + f'    mov {reg}, {"w14" if reg.startswith("w") else "x14"}'
        result.append(access.sub(replace, body))
    if dirty:
        assert result[-1].endswith('    ADVANCE')
        result[-1] = result[-1][:-len('    ADVANCE')] + f'    str x14, [x27, #{offset}]\n    ADVANCE'
    return result, True


def variant(recipe):
    name = recipe["handler"]
    bits, flags, d, a, b = (recipe[k] for k in ("bits", "flags", "d", "a", "b"))
    binary = {"Add", "Sub", "Adc", "Sbc", "And", "Or", "Xor", "Lsl", "Lsr", "Asr", "Ror", "Mul", "Udiv", "Sdiv"}
    prepared = {"AddPrepared", "SubPrepared", "AndPrepared", "OrPrepared", "XorPrepared", "Bitfield", "Unary"}
    if name in binary or name in {"PairLoad", "PairStore", "VectorPairLoad", "VectorPairStore"}:
        pass
    elif name in {"Move", "Keep", "VectorImmediate"}:
        flags, a, b = 0, 7, 7
    elif name in {"Operand", "Prepare"}:
        d, b = 7, 7
    elif name in prepared:
        b = 7
    elif name in {"Load", "VectorLoad", "LoadPrepared", "VectorLoadPrepared", "LoadWriteback", "VectorLoadWriteback"}:
        flags, b = 0, 7
    elif name in {"Store", "VectorStore", "StorePrepared", "VectorStorePrepared", "StoreWriteback", "VectorStoreWriteback"}:
        flags, d = 0, 7
    elif name in {"SignedLoad", "SignedLoadPrepared", "SignedLoadWriteback"}:
        b = 7
    else:
        return None
    return f"L{name}_{bits}_{flags}_{d}_{a}_{b}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--handlers", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--limit", type=int, default=256)
    parser.add_argument("--specialize", action="store_true")
    args = parser.parse_args()
    if args.limit < 1 or args.limit > 4096:
        parser.error("limit must be between 1 and 4096")
    profile = json.loads(args.profile.read_text())
    if profile.get("schema") != "operit.execution-diagnostics.v1":
        raise ValueError("unsupported diagnostic profile")
    handlers = args.handlers.read_text()
    candidates = collections.Counter()
    recipes, bodies, specialized = {}, {}, {}

    def handler_body(label):
        if label not in bodies:
            match = re.search(r"(?m)^" + re.escape(label) + r":\n(.*?)(?=^\.p2align 2\n|\Z)", handlers, re.S)
            body = match.group(1).rstrip() if match else ''
            bodies[label] = body if body.endswith('    ADVANCE') else None
        return bodies[label]

    for site in profile.get("top_sites", []):
        for mapping in site.get("mappings", []):
            operations = mapping["lowered_sequence"]
            for start in range(len(operations)):
                sequence = []
                for op in operations[start:start + 6]:
                    r = op.get("recipe")
                    name = variant(r) if r else None
                    if name is None or not handler_body(name):
                        break
                    body, guards = specialize(bodies[name], op) if args.specialize else (bodies[name], ())
                    item = (name, guards)
                    specialized[item] = body
                    sequence.append(item)
                    recipes.setdefault(name, r)
                    if len(sequence) >= 2:
                        candidates[tuple(sequence)] += site["samples"]
    selected = [sequence for sequence, _ in candidates.most_common(args.limit)]
    if not selected:
        raise ValueError("no supported sequences in profile")
    asm = ["// Generated from profile-selected static executor handlers."]
    cpp = ['#include "cpu/aarch64/block_graph.h"', '#include "cpu/aarch64/profile_superblocks.h"',
           '#include <unordered_map>', '#include <vector>', 'namespace operit::arm64 {', 'extern "C" {']
    cpp += [f'void operit_profile_block_{i}();' for i in range(len(selected))]
    input_type = ('struct Input { const void *entry; uint64_t immediate; uint16_t a,b,d; uint8_t mask; };'
                  if args.specialize else '')
    input_element = 'Input' if args.specialize else 'const void *'
    cpp += ['}', 'namespace {', input_type, f'struct Pattern {{ const void *output; std::vector<{input_element}> inputs; }};',
            'using Index = std::unordered_map<const void *,std::vector<Pattern>>;',
            'const Index &patterns() { static const Index index=[] { Index result;']
    promoted = 0
    for number, sequence in enumerate(selected):
        asm += ['.p2align 2', f'.globl SYM(operit_profile_block_{number})', f'SYM(operit_profile_block_{number}):',
                '.cfi_startproc', '.cfi_def_cfa x29, 224', '.cfi_offset x29, -224', '.cfi_offset x30, -216',
                '.cfi_offset x19, -208', '.cfi_offset x20, -200', '.cfi_offset x21, -192',
                '.cfi_offset x22, -184', '.cfi_offset x23, -176', '.cfi_offset x24, -168',
                '.cfi_offset x25, -160', '.cfi_offset x26, -152', '.cfi_offset x27, -144', '.cfi_offset x28, -136',
                '.cfi_offset 72, -128', '.cfi_offset 73, -112', '.cfi_offset 74, -96', '.cfi_offset 75, -80',
                '.cfi_offset 76, -64', '.cfi_offset 77, -48', '.cfi_offset 78, -32', '.cfi_offset 79, -16',
                '    DIAG_COUNT ED_EVENT_PROFILE_SUPERBLOCK_ENTRY']
        compiled = [specialized[item] for item in sequence]
        if args.specialize:
            compiled, did_promote = promote_shadow(compiled, [recipes[item[0]]['handler'] for item in sequence])
            promoted += did_promote
        for pos, item in enumerate(sequence):
            body = compiled[pos]
            local = re.findall(r'(?m)^(L\w+):', body)
            for name in sorted(set(local), key=len, reverse=True):
                body = re.sub(r'\b' + re.escape(name) + r'\b', f'{name}_profile_{number}_{pos}', body)
            if pos + 1 < len(sequence):
                body = body[:-len('    ADVANCE')] + '    add x28, x28, #OP_SIZE'
            asm.append(body)
        asm.append('.cfi_endproc')
        expressions = []
        for label, guards in sequence:
            r = recipes[label]
            entry = 'static_entry(StaticCode::{handler},{bits},{d},{a},{b},{flags})'.format(**r)
            if args.specialize:
                g = dict(guards)
                mask = sum(bit for key, bit in [('immediate', 1), ('a', 2), ('b', 4), ('d', 8)] if key in g)
                entry = '{{{},{}ULL,{},{},{},{}}}'.format(entry, g.get('immediate', 0), g.get('a', 0),
                                                          g.get('b', 0), g.get('d', 0), mask)
            expressions.append(entry)
        joined = ','.join(expressions)
        first = 'p.inputs.front().entry' if args.specialize else 'p.inputs.front()'
        cpp.append(f'    {{ Pattern p{{reinterpret_cast<const void*>(&operit_profile_block_{number}),{{{joined}}}}}; result[{first}].push_back(std::move(p)); }}')
    cpp += ['    return result; }(); return index; }', '}',
            'bool is_profile_superblock_entry(const void *entry) {',
            '    for(const auto &bucket:patterns())for(const auto &pattern:bucket.second)',
            '        if(pattern.output==entry)return true;',
            '    return false;', '}',
            'std::size_t apply_profile_superblocks(BasicBlock &block) {',
            '    auto &code=block.code;const auto &index=patterns();std::size_t installed=0;',
            '    for(size_t i=2;i<code.size();++i) {',
            '        const auto found=index.find(code[i].entry);if(found==index.end())continue;',
            '        for(const auto &p:found->second) {',
            '            if(p.inputs.size()>code.size()-i)continue;',
            ('            size_t n=0;for(;n<p.inputs.size();++n) { const auto &e=p.inputs[n]; const auto &o=code[i+n]; '
             'if(o.entry!=e.entry || ((e.mask&1) && o.immediate!=e.immediate) || ((e.mask&2) && o.a!=e.a) || '
             '((e.mask&4) && o.b!=e.b) || ((e.mask&8) && o.d!=e.d))break; }'
             if args.specialize else
             '            size_t n=0;for(;n<p.inputs.size() && code[i+n].entry==p.inputs[n];++n) {}'),
            '            if(n==p.inputs.size()) {code[i].entry=p.output;i+=n-1;++installed;break;}',
            '        }',
            '    }', '    return installed;', '}', '}']
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / 'profile_superblocks.inc').write_text('\n'.join(asm) + '\n')
    (args.output / 'profile_superblocks.cpp').write_text('\n'.join(cpp) + '\n')
    print(f"generated {len(selected)} superblocks from {len(candidates)} candidates; shadow-promoted={promoted}")


if __name__ == '__main__':
    main()
