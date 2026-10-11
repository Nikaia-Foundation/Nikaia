#!/usr/bin/env python3
"""Below-core Rust (non-stub) functions with level, Rust-only helpers and host needs.
usage: below2.py mod.rs g.json"""
import re, sys, json
src = open(sys.argv[1]).read().split("\n")
g = json.load(open(sys.argv[2]))
fns = g["fns"]; calls = g["calls"]; stub = g["stub"]; core = set(g["core"])
below = [f for f in g["below"] if not stub[f]]
B = set(below)
level = {}
def lv(f, seen=()):
    if f in level: return level[f]
    if f in seen: return 0
    deps = [x for x in calls[f] if x in B]
    level[f] = 0 if not deps else 1 + max(lv(x, seen + (f,)) for x in deps)
    return level[f]
HELP = r"crate::((?:contracts::)?(?:sync|order|emit|keeps|dsl|bounds|prove|send|ty|views|types|traits|build_time|comptime_run|tether|fold)::[a-z_A-Z]+)|\b(order::[a-z_]+|send::[a-z_]+|build_time::[a-z_]+|ty::[a-z_]+)\("
rows = []
for f in below:
    a, b = fns[f]
    body = "\n".join(re.sub(r"(^|\s)//.*$", "", l) for l in src[a:b + 1])
    helpers = sorted({(m.group(1) or m.group(2)) for m in re.finditer(HELP, body)} - {"contracts::ty::base", "ty::base", "contracts::ty::TEXT"})
    host = sorted(set(re.findall(r"self\.(parsed|beside|reads|here|newly|host\(\))", body)) - set())
    mut = "&mut self" in src[a] or "&mut self" in "\n".join(src[a:a+4])
    callers = sorted(x for x in g["fns"] if f in calls[x] and x in core)
    rows.append((lv(f), fns[f][0], f, b - a + 1, mut, helpers, host, callers))
rows.sort()
tot = 0
for r in rows:
    tot += r[3]
    print(f"{r[0]} L{r[1]+1:6} {r[3]:4} {'mut' if r[4] else 'ref'} {r[2]:44} help={','.join(r[5])} host={','.join(r[6])} core-callers={len(r[7])}")
print(len(rows), tot)
