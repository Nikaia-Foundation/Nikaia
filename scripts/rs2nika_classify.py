#!/usr/bin/env python3
"""Classify every added Nikaia code line of a self-hosting commit.

S  syntactic: rs2nika's draft of the moved Rust has the same line (whitespace
   and a trailing comma ignored).
A  adaptation: not in the draft, but every name in it is a name of the moved
   Rust (after stripping module prefixes): same logic, other form.
N  new design: it introduces a name the Rust did not have (a helper, a field,
   a loop index, a struct of state).
usage: classify2.py x_<commit>.json [--dump]
"""
import json, re, sys, random
from collections import Counter
sys.path.insert(0, __file__.rsplit('/', 1)[0])
import rs2nika

KW = set('''fn let mut return match else if for in while ref self sync throws null true false pub impl struct enum use
collections BTreeMap BTreeSet Vec String i64 i32 u8 u32 u64 f64 bool scalar clone len push world words resolve CheckWorld EmitWorld
CheckState EmitState panic break continue as std Self is_empty contains contains_key get insert extend join collect chars
trim starts_with ends_with strip_prefix count filter map iter to_string text node empty Ty Expr Stmt Item Pattern Type
Block Spanned Symbol winnow_grammar remove pop sort keys values span start end try catch throw f return_ checked'''.split())


def norm(l):
    l = l.strip().rstrip(',').strip()
    l = re.sub(r'\s+', ' ', l)
    l = re.sub(r'f"([^"{}]*)"', r'"\1"', l)  # f"x" with no hole == "x"
    return l


def idents(l):
    l = re.sub(r'"(?:\\.|[^"\\])*"', '""', l)
    return set(re.findall(r'\b[A-Za-z_]\w*\b', l))


def base(n):
    for p in ('emit_core_', 'emit_', 'check_', 'Check', 'Emit', 'Fact', 'pattern_'):
        if n.startswith(p) and len(n) > len(p):
            return n[len(p):]
    return n


def main():
    d = json.load(open(sys.argv[1]))
    profile = 'emit' if 'emit' in d['subject'].split(':')[0] else 'check'
    rust = '\n'.join(f['text'] for f in d['rust_fns'])
    extra = '\n'.join(d['rust_removed'])
    draft = rs2nika.convert(rust, profile)
    dl = Counter(norm(l) for l in draft.split('\n') if l.strip() and not l.strip().startswith('//'))
    rid = idents(rust + '\n' + extra + '\n' + draft)
    rid_low = {x.lower() for x in rid}
    res = []
    for a in d['nika_added']:
        n = norm(a['text'])
        if n in ('}', '{', ''):
            # closing braces: syntactic by construction
            res.append(('S', a)); continue
        if dl[n] > 0:
            dl[n] -= 1
            res.append(('S', a)); continue
        new = [i for i in idents(a['text']) if i not in KW and i not in rid and base(i) not in rid and i.lower() not in rid_low and base(i).lower() not in rid_low]
        res.append(('N' if new else 'A', a, new))
    c = Counter(r[0] for r in res)
    tot = sum(c.values())
    print(f"{d['commit']} {profile} rust_removed={len(d['rust_removed'])} nika_added={tot} " +
          ' '.join(f"{k}={c[k]} ({100*c[k]/tot:.0f}%)" for k in 'SAN'))
    if '--dump' in sys.argv:
        for r in res:
            print(r[0], r[1]['text'].strip()[:150], ('  NEW:' + ','.join(r[2])) if len(r) > 2 and r[2] else '')
    json.dump([[r[0], r[1]['text']] for r in res], open(sys.argv[1].replace('x_', 'cls_'), 'w'))


main()
