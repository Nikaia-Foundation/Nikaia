#!/usr/bin/env python3
"""How much of an rs2nika draft passes `nikaia lower-std` as it stands.

Puts the draft into the scratch worktree as tools/draft_x.nika, runs lower-std,
drops every item an error points into (a parse error stops at the first, a
check error lists all), and repeats until what is left passes.
usage: passrate.py draft.nika [passing_out.nika]   -> items/lines passing, and the first error of each dropped item
"""
import re, subprocess, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rs2nika

E = os.path.dirname(os.path.abspath(__file__))
WT = E + '/wt'
DST = WT + '/crates/nikaia-std/src/tools/draft_x.nika'


def code(ls):
    return sum(1 for l in ls if l.strip() and not l.strip().startswith('//'))


def build(its):
    out, line_of = ['use std::collections', ''], []
    free = [(i, ls) for i, (k, ls) in enumerate(its) if k == 'item']
    meth = [(i, ls) for i, (k, ls) in enumerate(its) if k == 'method']
    owner = {}
    for i, ls in free:
        for l in ls:
            out.append(l); owner[len(out)] = i
        out.append('')
    if meth:
        out.append('impl CheckState {' if 'CheckWorld' in str(meth) else 'impl EmitState {')
        for i, ls in meth:
            for l in ls:
                out.append('    ' + l if l.strip() else ''); owner[len(out)] = i
            out.append('')
        out.append('}')
    return '\n'.join(out) + '\n', owner


def run():
    r = subprocess.run([E + '/nikaia', 'lower-std', '--sysroot', WT + '/crates'], capture_output=True, text=True, cwd=WT)
    return r.stdout + r.stderr


def main():
    its = rs2nika.items(open(sys.argv[1]).read())
    total_items, total_lines = len(its), sum(code(ls) for _, ls in its)
    alive = dict(enumerate(its))
    dropped = {}
    for _ in range(60):
        if not alive:
            break
        keys = sorted(alive)
        text, owner = build([alive[k] for k in keys])
        open(DST, 'w').write(text)
        out = run()
        lines = [int(m) for m in re.findall(r'draft_x\.nika:(\d+):\d+', out)]
        m = re.search(r'draft_x\.nika: (.*?) \(line (\d+), column', out)
        if m:
            lines.append(int(m[2]))
        if not lines:
            if 'Error' in out and 'draft_x' in out:
                print('unattributed error:', out[:400]); break
            break
        bad = set()
        for ln in lines:
            j = owner.get(ln)
            if j is None:
                near = [l for l in owner if l <= ln]
                j = owner[max(near)] if near else 0
            bad.add(keys[j])
        msgs = re.findall(r'(error\[NK\d+\]: [^\n]*)\n\s*--> [^\n]*draft_x\.nika:(\d+)', out)
        for b in bad:
            first = (m[1] if m else None)
            for msg, ln in msgs:
                if keys[owner.get(int(ln), 0)] == b:
                    first = msg; break
            dropped[b] = first or out.strip().split('\n')[0][:200]
            del alive[b]
    passed = sum(code(alive[k][1]) for k in alive)
    print(f'items passing {len(alive)}/{total_items}, code lines passing {passed}/{total_lines} ({100*passed/max(total_lines,1):.0f}%)')
    for k in sorted(dropped):
        name = re.search(r'fn (\w+)', '\n'.join(its[k][1]))
        print(f'  dropped {name[1] if name else k}: {dropped[k][:170]}')
    if len(sys.argv) > 2 and alive:
        open(sys.argv[2], 'w').write(build([alive[k] for k in sorted(alive)])[0])
    os.remove(DST)
    subprocess.run(['git', '-C', WT, 'checkout', '-q', 'crates/nikaia-std/src/tools/package.rs'])


main()
