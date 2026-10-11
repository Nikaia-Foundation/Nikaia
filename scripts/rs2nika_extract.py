#!/usr/bin/env python3
"""Extract what one self-hosting commit moved: the Rust functions it emptied
and the Nikaia lines it added.

usage: extract.py REPO COMMIT > out.json
"""
import json, re, subprocess, sys

REPO, C = sys.argv[1], sys.argv[2]


def git(*a):
    return subprocess.run(['git', '-C', REPO, *a], capture_output=True, text=True, check=True).stdout


def code(line):
    s = line.strip()
    return bool(s) and not s.startswith('//')


def hunks(path_glob):
    """{file: (removed_line_numbers_in_parent, added_line_numbers_in_child)}"""
    out = {}
    cur = None
    for l in git('diff', '-U0', C + '^', C, '--', path_glob).split('\n'):
        if l.startswith('+++ b/'):
            cur = l[6:]
            out[cur] = (set(), set())
        elif l.startswith('@@') and cur:
            m = re.match(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@', l)
            a, an, b, bn = int(m[1]), int(m[2] or 1), int(m[3]), int(m[4] or 1)
            out[cur][0].update(range(a, a + an))
            out[cur][1].update(range(b, b + bn))
    return out


FN = re.compile(r'^(\s*)(?:pub(?:\([a-z]+\))? )?(?:const )?(?:fn|struct|enum) ([A-Za-z_][A-Za-z0-9_]*)')


def functions(text):
    """[(name, start_line(1-based), end_line, lines)] by brace matching."""
    lines = text.split('\n')
    res = []
    i = 0
    while i < len(lines):
        m = FN.match(lines[i])
        if not m:
            i += 1
            continue
        ind = m[1]
        opened = not lines[i].rstrip().endswith(';')
        j = i
        if opened:
            j = i + 1
            while j < len(lines) and lines[j].rstrip() != ind + '}' and not (lines[i].rstrip().endswith('}') and j == i + 1):
                j += 1
            if lines[i].rstrip().endswith('}'):
                j = i
        j = min(j, len(lines) - 1)
        res.append((m[2], i + 1, j + 1, lines[i:j + 1]))
        i = j + 1 if opened else i + 1
    return res


def main():
    out = {'commit': C, 'subject': git('log', '-1', '--format=%s', C).strip(),
           'rust_fns': [], 'rust_removed': [], 'nika_added': [], 'nika_files': {}}
    for f, (rem, _) in hunks('crates/nikaia/src').items():
        if not f.endswith('.rs'):
            continue
        old = git('show', f'{C}^:{f}')
        olines = old.split('\n')
        out['rust_removed'] += [olines[n - 1] for n in sorted(rem) if n - 1 < len(olines) and code(olines[n - 1])]
        for name, a, b, body in functions(old):
            cl = [n for n in range(a, b + 1) if n - 1 < len(olines) and code(olines[n - 1])]
            gone = [n for n in cl if n in rem]
            if len(cl) >= 3 and len(gone) >= 0.5 * len(cl):
                out['rust_fns'].append({'file': f, 'name': name, 'text': '\n'.join(body), 'code': len(cl), 'removed': len(gone)})
    for f, (_, add) in hunks('crates/nikaia-std/src/tools').items():
        if not f.endswith('.nika'):
            continue
        new = git('show', f'{C}:{f}').split('\n')
        out['nika_added'] += [{'file': f, 'line': n, 'text': new[n - 1]} for n in sorted(add) if n - 1 < len(new) and code(new[n - 1])]
        out['nika_files'][f] = '\n'.join(new)
    json.dump(out, sys.stdout, indent=1)


main()
