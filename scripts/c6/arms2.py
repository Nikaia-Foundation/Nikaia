#!/usr/bin/env python3
"""Arms of a match in a Checker method with core / Rust-below callees and host needs.
usage: arms2.py mod.rs g.json method indent"""
import re, sys, json
src = open(sys.argv[1]).read().split("\n")
g = json.load(open(sys.argv[2])); name = sys.argv[3]; ind = int(sys.argv[4])
a, b = g["fns"][name]
core = set(g["core"]); stub = g["stub"]; kind = g["kind"]; below = set(g["below"])
pat = re.compile(r"^" + " " * ind + r"(?:\| )?((?:Expr|Stmt)::[A-Za-z]+)")
arms = [(i, pat.match(src[i]).group(1)) for i in range(a, b) if pat.match(src[i])]
names = set(g["fns"])
for k, (i, v) in enumerate(arms):
    j = arms[k + 1][0] if k + 1 < len(arms) else b
    body = "\n".join(re.sub(r"(^|\s)//.*$", "", l) for l in src[i:j])
    called = set(re.findall(r"(?:self|Self)\s*(?:\.|::)\s*([a-z_0-9]+)\s*\(", body))
    called |= {m for m in re.findall(r"(?<![\w.:])([a-z_0-9]+)\s*\(", body) if m in names and kind.get(m) == "f"}
    c = sorted(x for x in called if x in core)
    r = sorted(x for x in called if x in names and x not in core and not stub.get(x))
    host = sorted(set(re.findall(r"self\.(parsed|beside|reads|here|newly|host\(\))", body)))
    crate = sorted(set(re.findall(r"crate::([a-z_]+::[a-z_A-Z]+)", body)))
    code = sum(1 for l in src[i:j] if l.strip() and not l.strip().startswith("//"))
    print(f"L{i+1:6} {j-i:4} code={code:4} {v:22} core={','.join(c)} | rust={','.join(r)} | host={','.join(host)} {','.join(crate)}")
