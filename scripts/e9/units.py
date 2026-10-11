#!/usr/bin/env python3
"""Check a unit order for E9: every in-core callee of a moved method is moved
earlier, in the same unit, or is a hub reached through EmitHost.
usage: units.py g.json mod.rs"""
import json, re, sys
g = json.load(open(sys.argv[1])); src = open(sys.argv[2]).read().split("\n")
core = set(g["core"]); calls = g["calls"]; fns = g["fns"]
HUBS = {"expr", "block_opening_with"}

def arms(name):
    a, b = fns[name]
    pat = re.compile(r"^            (?:\| )?((?:Expr|Stmt)::[A-Za-z]+)")
    out = []
    for i in range(a, b):
        m = pat.match(src[i])
        if m: out.append((i, m.group(1)))
    res = {}
    for k, (i, v) in enumerate(out):
        j = out[k + 1][0] if k + 1 < len(out) else b
        body = "\n".join(src[i:j])
        cs = set(re.findall(r"self\.([a-z_0-9]+)\(", body)) & core
        r = res.setdefault(v, [0, set()]); r[0] += j - i; r[1] |= cs
    return res

EA = arms("expr_as_written"); SA = arms("a_stmt")
node_lines = {f: fns[f][1] - fns[f][0] for f in core}
node_calls = {f: set(calls[f]) & core for f in core}
for v, (n, cs) in EA.items(): node_lines["E:" + v] = n; node_calls["E:" + v] = cs
for v, (n, cs) in SA.items(): node_lines["S:" + v] = n; node_calls["S:" + v] = cs

U = [
 ("U2 emit_core_leaves.nika", "operand bare_range index_expr key postfix_base nested handed_back concatenated joined_name format_string string dsl_parameters method_options guard_written grammar_entry template_segments template block if_expr"),
 ("U3 emit_core_reads.nika", "hold_owned_receiver hold_indices_read_from_the_target hold_temporaries against_a_length index_read a_field_call receiver overlap select"),
 ("U4 emit_core_match.nika", "pattern_written pattern_lending match_pattern arm_body_led arm_body_opening arm_body match_over_a_sum E:Expr::Match"),
 ("U5 emit_core_args.nika", "args called"),
 ("U6 emit_core_calls.nika", "call_inner call method_call E:Expr::Call E:Expr::MethodCall"),
 ("U7 emit_core_places.nika", "E:Expr::SafeMethod E:Expr::Field E:Expr::SafeField E:Expr::Index E:Expr::Cast E:Expr::StructLit E:Expr::With E:Expr::Tuple E:Expr::ListLit"),
 ("U8 emit_core_flow.nika", "E:Expr::Closure E:Expr::Binary E:Expr::Coalesce E:Expr::TryCatch E:Expr::Try E:Expr::Return E:Expr::Break E:Expr::Throw E:Expr::Spawn"),
 ("U9 emit_core_stmt.nika", " ".join("S:" + v for v in SA)),
 ("U10 emit_core_expr.nika", " ".join("E:" + v for v in EA if v not in ("Expr::Match","Expr::Call","Expr::MethodCall","Expr::SafeMethod","Expr::Field","Expr::SafeField","Expr::Index","Expr::Cast","Expr::StructLit","Expr::With","Expr::Tuple","Expr::ListLit","Expr::Coalesce","Expr::Closure","Expr::Binary","Expr::TryCatch","Expr::Try","Expr::Return","Expr::Break","Expr::Throw","Expr::Spawn")) + " expr expr_after_the_copy expr_as_written block_opening_with block_plain stmt a_stmt"),
]
moved = set(); ok = True; seen = set()
for name, members in U:
    ms = members.split()
    for m in ms:
        if m not in node_lines: print("unknown", m); ok = False
        seen.add(m)
    unit = set(ms)
    later = set()
    for m in ms:
        for c in node_calls[m]:
            if c in moved or c in unit or c in HUBS: continue
            if c in ("expr_as_written", "a_stmt"): continue  # dispatchers stay Rust till U10
            later.add(c)
    lines = sum(node_lines[m] for m in ms if not (m in ("expr_as_written", "a_stmt")))
    if "expr_as_written" in ms: lines += node_lines["expr_as_written"] - sum(n for n, _ in EA.values())
    if "a_stmt" in ms: lines += node_lines["a_stmt"] - sum(n for n, _ in SA.values())
    hosts = sorted({c for m in ms for c in node_calls[m] if c in HUBS and c not in moved and c not in unit})
    print(f"{name:28} lines={lines:5} host={hosts} {'LATER:'+','.join(sorted(later)) if later else ''}")
    moved |= unit
missing = (set(node_lines) - seen) - {"expr_as_written", "a_stmt"}
print("not placed:", sorted(m for m in missing if not m.startswith(("E:", "S:")) or True) if missing else "none")
