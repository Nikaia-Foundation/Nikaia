#!/usr/bin/env python3
"""Arms of the top-level match in a method: arms.py mod.rs method_name arm_indent"""
import re, sys, json
src = open(sys.argv[1]).read().split("\n")
name = sys.argv[2]
ind = int(sys.argv[3]) if len(sys.argv) > 3 else 12
g = json.load(open(sys.argv[4])) if len(sys.argv) > 4 else None
start = next(i for i, l in enumerate(src) if re.match(r"^    fn " + name + r"\b", l))
end = next(i for i in range(start + 1, len(src)) if re.match(r"^    (pub )?fn |^}", src[i]))
arms = []
pat = re.compile(r"^" + " " * ind + r"(?:\| )?([A-Z][A-Za-z:]*(?:::[A-Z][A-Za-z]*)?)")
for i in range(start, end):
    m = pat.match(src[i])
    if m and not src[i].strip().startswith("//"):
        arms.append((i, m.group(1)))
names = set(g["fns"]) if g else set()
tot = 0
for k, (i, a) in enumerate(arms):
    j = arms[k + 1][0] if k + 1 < len(arms) else end
    body = "\n".join(src[i:j])
    called = sorted(set(re.findall(r"self\.([a-z_0-9]+)\(", body)) & names) if g else []
    code = sum(1 for l in src[i:j] if l.strip() and not l.strip().startswith("//"))
    tot += j - i
    print(f"L{i+1:6} {j-i:5} code={code:4} {a:28} {','.join(c for c in called if c in g['core'])}")
print("arms", len(arms), "lines", tot, "method", end - start)
