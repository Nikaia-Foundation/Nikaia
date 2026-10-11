#!/usr/bin/env python3
"""Call graph of Emitter methods in emit/mod.rs; SCCs; core analysis.
usage: callgraph.py mod.rs [mode]"""
import re, sys, json
from collections import defaultdict

src = open(sys.argv[1]).read().split("\n")
mode = sys.argv[2] if len(sys.argv) > 2 else "summary"

# find impl blocks of Emitter (top-level 'impl<'p> Emitter<'p> {' / 'impl Emitter<'_> {')
fns = {}  # name -> (start, end)
order = []
in_impl = False
cur = None
for i, line in enumerate(src):
    if re.match(r"^impl(<[^>]*>)? Emitter<", line):
        in_impl = True
        continue
    if in_impl and line.startswith("}"):
        if cur:
            fns[cur] = (fns[cur][0], i)
            cur = None
        in_impl = False
        continue
    if in_impl:
        m = re.match(r"^    (?:pub(?:\([a-z]+\))? )?fn ([a-z_0-9]+)", line)
        if m:
            if cur:
                fns[cur] = (fns[cur][0], i)
            cur = m.group(1)
            if cur in fns:
                cur = cur + "#2"
            fns[cur] = (i, None)
            order.append(cur)

names = set(fns)
calls = defaultdict(set)
for f, (a, b) in fns.items():
    body = "\n".join(src[a + 1:b])
    for m in re.finditer(r"(?:self|Self|emitter|this)\s*(?:\.|::)\s*([a-z_0-9]+)\s*[(:<]", body):
        n = m.group(1)
        if n in names and n != f:
            calls[f].add(n)
    # method refs passed as fn values e.g. Self::foo without parens
    for m in re.finditer(r"Self::([a-z_0-9]+)\b", body):
        n = m.group(1)
        if n in names and n != f:
            calls[f].add(n)
    if re.search(r"(?:self|Self)\s*(?:\.|::)\s*" + re.escape(f) + r"\s*\(", body):
        calls[f].add(f)

sys.setrecursionlimit(100000)
# Tarjan
idx = {}; low = {}; st = []; on = set(); sccs = []; c = [0]
def strong(v):
    idx[v] = low[v] = c[0]; c[0] += 1; st.append(v); on.add(v)
    for w in calls[v]:
        if w not in idx:
            strong(w); low[v] = min(low[v], low[w])
        elif w in on:
            low[v] = min(low[v], idx[w])
    if low[v] == idx[v]:
        comp = []
        while True:
            w = st.pop(); on.discard(w); comp.append(w)
            if w == v: break
        sccs.append(comp)
for v in order:
    if v not in idx: strong(v)

def lines(f):
    a, b = fns[f]; return b - a

big = max(sccs, key=len)
core = set(big)
if mode == "summary":
    print("methods", len(fns), "sccs>1:", [(len(s), sum(map(lines, s))) for s in sccs if len(s) > 1])
    print("core", len(core), "lines", sum(map(lines, core)))
    for f in sorted(core, key=lambda f: fns[f][0]):
        inn = [g for g in core if f in calls[g]]
        out = [g for g in calls[f] if g in core and g != f]
        below = [g for g in calls[f] if g not in core]
        print(f"{f:32} L{fns[f][0]+1:6} {lines(f):5}  in={len(inn):2} out={len(out):2} below={len(below):2}  -> {','.join(sorted(out))}")
elif mode == "above":
    # who outside core calls core
    for f in fns:
        if f not in core and calls[f] & core:
            print(f, "->", sorted(calls[f] & core))
elif mode == "json":
    json.dump({"fns": {f: list(fns[f]) for f in fns}, "calls": {f: sorted(calls[f]) for f in fns}, "core": sorted(core)}, open(sys.argv[3], "w"))
elif mode == "cut":
    # remove hub nodes given in argv[3] (comma list) and show residual SCCs of core
    hubs = set(sys.argv[3].split(","))
    sub = {f: {g for g in calls[f] if g in core and g not in hubs} for f in core if f not in hubs}
    idx.clear(); low.clear(); st.clear(); on.clear(); c[0] = 0
    res = []
    def strong2(v):
        idx[v] = low[v] = c[0]; c[0] += 1; st.append(v); on.add(v)
        for w in sub[v]:
            if w not in idx:
                strong2(w); low[v] = min(low[v], low[w])
            elif w in on:
                low[v] = min(low[v], idx[w])
        if low[v] == idx[v]:
            comp = []
            while True:
                w = st.pop(); on.discard(w); comp.append(w)
                if w == v: break
            res.append(comp)
    for v in sub:
        if v not in idx: strong2(v)
    # res is in reverse topological order (leaves first)
    for comp in res:
        print(len(comp), sum(map(lines, comp)), sorted(comp))
