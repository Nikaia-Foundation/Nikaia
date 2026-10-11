#!/usr/bin/env python3
"""Call graph of Checker methods + free fns in check/mod.rs; SCC; cut; arms.
usage: cg.py mod.rs mode [args]"""
import re, sys, json
from collections import defaultdict
sys.setrecursionlimit(100000)
src = open(sys.argv[1]).read().split("\n")
mode = sys.argv[2] if len(sys.argv) > 2 else "summary"

fns = {}; order = []; kind = {}
impl = None
for i, line in enumerate(src):
    m = re.match(r"^impl(<[^>]*>)? ([A-Za-z]+)", line)
    if m:
        impl = m.group(2); continue
    if line.startswith("}"):
        impl = None
    m4 = re.match(r"^    (?:pub(?:\([a-z]+\))? )?fn ([a-z_0-9]+)", line)
    m0 = re.match(r"^(?:pub(?:\([a-z]+\))? )?fn ([a-z_0-9]+)", line)
    if m4 and impl == "Checker":
        n = m4.group(1); ind = 4
    elif m0:
        n = m0.group(1); ind = 0
    else:
        continue
    k = i
    while not (src[k].rstrip().endswith("{") or src[k].rstrip().endswith(";")): k += 1
    if src[k].rstrip().endswith(";"): continue
    j = k
    while src[j] != " " * ind + "}": j += 1
    if n in fns: n = n + "#2"
    fns[n] = (i, j); order.append(n); kind[n] = "m" if ind == 4 else "f"

names = set(fns)
calls = defaultdict(set)
for f, (a, b) in fns.items():
    body = "\n".join(re.sub(r"(^|\s)//.*$", "", l) for l in src[a + 1:b])
    for m in re.finditer(r"(?:self|Self|Checker|checker|this|c)\s*(?:\.|::)\s*([a-z_0-9]+)\b", body):
        n = m.group(1)
        if n in names and kind[n] == "m": calls[f].add(n)
    for m in re.finditer(r"(?<![\w.:])([a-z_0-9]+)\s*(?:::<[^>]*>)?\(", body):
        n = m.group(1)
        if n in names and kind[n] == "f": calls[f].add(n)
    calls[f].discard(f)

def lines(f): a, b = fns[f]; return b - a + 1
def code(f):
    a, b = fns[f]; return sum(1 for l in src[a:b+1] if l.strip() and not l.strip().startswith("//"))
def is_stub(f):
    a, b = fns[f]; body = "\n".join(src[a:b+1])
    return code(f) <= 9 and bool(re.search(r"self\.s\b|\bcheck_\w+::|nikaia_std::|resolve_call|resolve_method|names_of\(|pattern_names\(|id_of\(|self\.host\(\)", body))

def tarjan(nodes, edges):
    idx = {}; low = {}; st = []; on = set(); out = []; c = [0]
    def go(v):
        idx[v] = low[v] = c[0]; c[0] += 1; st.append(v); on.add(v)
        for w in edges.get(v, ()):
            if w not in nodes: continue
            if w not in idx: go(w); low[v] = min(low[v], low[w])
            elif w in on: low[v] = min(low[v], idx[w])
        if low[v] == idx[v]:
            comp = []
            while True:
                w = st.pop(); on.discard(w); comp.append(w)
                if w == v: break
            out.append(comp)
    for v in nodes:
        if v not in idx: go(v)
    return out  # reverse topological (leaves first)

sccs = tarjan(order, calls)
core = set(max(sccs, key=len))
# reach: everything the core reaches (below)
def reach(starts, edges, stop=set()):
    seen = set(); todo = list(starts)
    while todo:
        v = todo.pop()
        for w in edges.get(v, ()):
            if w not in seen and w not in stop: seen.add(w); todo.append(w)
    return seen
below = reach(core, calls) - core
rev = defaultdict(set)
for f in calls:
    for g in calls[f]: rev[g].add(f)
above = reach(core, rev) - core

if mode == "summary":
    print("fns", len(fns), "big sccs", [(len(s), sum(map(lines, s))) for s in sccs if len(s) > 1])
    print("core", len(core), "lines", sum(map(lines, core)), "below", len(below), sum(map(lines, below)), "above", len(above), sorted(above))
    for f in sorted(core, key=lambda f: fns[f][0]):
        inn = sorted(g for g in core if f in calls[g])
        out = sorted(g for g in calls[f] if g in core)
        bl = [g for g in calls[f] if g not in core]
        print(f"{f:30} L{fns[f][0]+1:6} {lines(f):5} in={len(inn):2} out={len(out):2} below={len(bl):3} -> {','.join(out)}")
elif mode == "below":
    # below-core functions that are not stubs (still Rust), leaves-first order
    sub = {f: calls[f] & below for f in below}
    topo = tarjan(sorted(below, key=lambda f: fns[f][0]), sub)
    tot = 0; n = 0
    for comp in topo:
        for f in comp:
            if is_stub(f): continue
            n += 1; tot += lines(f)
            rust = sorted(g for g in calls[f] if g in below and not is_stub(g))
            print(f"{f:40} {kind[f]} L{fns[f][0]+1:6} {lines(f):5} {'SCC'+str(len(comp)) if len(comp)>1 else ''} rust-callees={','.join(rust)}")
    print("non-stub below", n, tot)
elif mode == "cut":
    hubs = set(sys.argv[3].split(","))
    sub = {f: {g for g in calls[f] if g in core and g not in hubs} for f in core if f not in hubs}
    for comp in tarjan(sorted(sub, key=lambda f: fns[f][0]), sub):
        print(len(comp), sum(map(lines, comp)), sorted(comp))
elif mode == "mincut":
    # greedy: try every subset of size 1..3 among core; report those making DAG
    import itertools
    cand = sorted(core, key=lambda f: -len([g for g in core if f in calls[g]]))
    for k in (1, 2, 3, 4):
        found = []
        for hs in itertools.combinations(sorted(core), k):
            hubs = set(hs)
            sub = {f: {g for g in calls[f] if g in core and g not in hubs} for f in core if f not in hubs}
            if all(len(c) == 1 for c in tarjan(sorted(sub), sub)): found.append(hs)
        if found:
            for hs in found: print(k, hs)
            break
elif mode == "json":
    json.dump({"fns": fns, "calls": {f: sorted(calls[f]) for f in fns}, "core": sorted(core),
               "below": sorted(below), "above": sorted(above), "kind": kind,
               "stub": {f: is_stub(f) for f in fns}}, open(sys.argv[3], "w"))
