#!/usr/bin/env python3
"""Check a unit order for C-6: every in-core callee of a moved node is moved
earlier, in the same unit, or is a hub reached through CheckHost.
usage: units_c.py g.json mod.rs"""
import json, re, sys
g = json.load(open(sys.argv[1])); src = open(sys.argv[2]).read().split("\n")
core = set(g["core"]); fns = g["fns"]

def arms(name):
    a, b = fns[name]
    pat = re.compile(r"^            (?:\| )?((?:Expr|Stmt)::[A-Za-z]+)")
    out = [(i, pat.match(src[i]).group(1)) for i in range(a, b) if pat.match(src[i])]
    res = {}
    for k, (i, v) in enumerate(out):
        j = out[k + 1][0] if k + 1 < len(out) else b
        body = "\n".join(re.sub(r"(^|\s)//.*$", "", l) for l in src[i:j])
        cs = set(re.findall(r"self\.([a-z_0-9]+)\(", body)) & core
        r = res.setdefault(v.split("::")[1], [0, set()]); r[0] += j - i; r[1] |= cs
    return res

EA = arms("value_of")
lines = {f: fns[f][1] - fns[f][0] + 1 for f in core}
calls = {f: set(g["calls"][f]) & core for f in core}
for v, (n, cs) in EA.items():
    lines["E:" + v] = n; calls["E:" + v] = cs
U = [
 ("U-c0 check_core_pilot", "grammar_call a_compound_write_to_a_map_slot an_instantiation a_mut_parameter_given_away"),
 ("U-c1 check_core_leaves", "a_field_of_a_borrowed_subject a_field_of_a_lent_place_handed_back a_kept_lambda literal_by_use holes lambda returns an_assert"),
 ("U-c2 check_core_comptime", "comptime_binding a_block_over_a_grammar"),
 ("U-c3 check_core_args", "arguments_given locks list_literal a_called_field"),
 ("U-c4 check_core_call", "call E:Call E:ListLit E:Tuple"),
 ("U-c5 check_core_call_on", "call_on a_safe_call"),
 ("U-c6 check_core_method", "E:SafeMethod E:MethodCall"),
 ("U-c7 check_core_stmt", "stmt block"),
 ("U-c8 check_core_flow", "E:Match E:If E:Select E:Overlap E:TryCatch E:Block E:Unsafe"),
 ("U-c9 check_core_places", "E:Field E:StructLit E:With E:Index E:Cast E:Range"),
 ("U-c10 check_core_ops", "E:Binary E:Coalesce E:Closure E:Spawn"),
 ("U-c11 check_core_expr", " ".join("E:" + v for v in EA if v not in
    "Call ListLit Tuple SafeMethod MethodCall Match If Select Overlap TryCatch Block Unsafe Field StructLit With Index Cast Range Binary Coalesce Closure Spawn".split()) + " value_of expr"),
]
HUBS = {"expr", "block"}
moved = set(); seen = set()
for name, members in U:
    ms = members.split(); unit = set(ms)
    for m in ms:
        if m not in lines: print("unknown", m)
        seen.add(m)
    later = sorted({c for m in ms for c in calls[m] if c not in moved | unit | HUBS | {"value_of"}})
    n = sum(lines[m] for m in ms if m != "value_of")
    if "value_of" in ms: n += 5  # the dispatcher itself
    hubs = sorted({c for m in ms for c in calls[m] if c in HUBS and c not in moved | unit})
    print(f"{name:26} lines={n:5} host={','.join(hubs)} {'LATER:' + ','.join(later) if later else ''}")
    moved |= unit
    if "block" in unit: HUBS.discard("block")
missing = set(lines) - seen
print("not placed:", sorted(missing) or "none")

# needs: Rust (non-stub) below-core functions each unit calls directly
if len(sys.argv) > 3:
    stub = g["stub"]; below = set(g["below"]); names = set(fns)
    grp = {}
    import re as _re
    for l in open(sys.argv[3]):
        m = _re.match(r"(\d) L\s*\d+\s+\d+ (?:ref|mut) (\S+)", l)
        if m: grp[m.group(2)] = m.group(1)
    def direct(m):
        if m.startswith("E:"):
            return set()
        a, b = fns[m]
        body = "\n".join(_re.sub(r"(^|\s)//.*$", "", l) for l in src[a:b+1])
        cs = set(_re.findall(r"(?:self|Self)\s*(?:\.|::)\s*([a-z_0-9]+)\s*\(", body)) | set(_re.findall(r"(?<![\w.:])([a-z_0-9]+)\s*\(", body))
        return {c for c in cs if c in names and c not in core and not stub[c] and c not in ("host", "others")}
    armsrc = {}
    a, b = fns["value_of"]
    pat = _re.compile(r"^            (?:\| )?Expr::([A-Za-z]+)")
    heads = [(i, pat.match(src[i]).group(1)) for i in range(a, b) if pat.match(src[i])]
    for k, (i, v) in enumerate(heads):
        j = heads[k + 1][0] if k + 1 < len(heads) else b
        body = "\n".join(_re.sub(r"(^|\s)//.*$", "", l) for l in src[i:j])
        cs = set(_re.findall(r"self\.([a-z_0-9]+)\(", body)) | set(_re.findall(r"(?<![\w.:])([a-z_0-9]+)\s*\(", body))
        armsrc.setdefault(v, set()).update(c for c in cs if c in names and c not in core and not stub[c] and c not in ("host", "others"))
    for name, members in U:
        need = set()
        for m in members.split():
            need |= armsrc.get(m[2:], set()) if m.startswith("E:") else direct(m)
        print(f"{name:26} needs {len(need)}: {' '.join(sorted(need))}")
