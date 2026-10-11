#!/usr/bin/env python3
"""Ledger of what the self-hosting agents cost against what they moved.

Two sources: the agents' transcripts (tokens, turns, active time) and the git
history of `develop` (Rust lines gone, Nikaia lines added, per commit). A
line is counted as `scripts/self_hosting.py` counts it: not blank, not only a
comment. Counts, never wall-clock claims about the machine: the minutes are
active minutes of the agent (gaps over 10 minutes, a build waiting, are cut).

Usage:
    scripts/tokenspeed.py AGENTS_DIR [SINCE_COMMIT]   print the two tables

AGENTS_DIR is the session's `subagents` directory under ~/.claude/projects.
The output is pasted into docs/tokenspeed.md under a dated heading.
"""

import json
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
from self_hosting import COMPILER_NIKA  # noqa: E402  the files that count as the toolchain's Nikaia
GAP = 600  # seconds; a longer pause is a build or a stop, not work


def git(*a):
    return subprocess.run(["git", "-C", str(ROOT), *a], capture_output=True, text=True).stdout


def code_lines(text):
    return sum(1 for l in text.splitlines() if l.strip() and not l.strip().startswith("//"))


def counts(commit):
    """(rust, nikaia) code lines at a commit."""
    rust = nika = 0
    for line in git("ls-tree", "-r", commit, "--", "crates/nikaia/src", "crates/nikaia-std/src/tools").splitlines():
        meta, path = line.split("\t")
        blob = meta.split()[2]
        if path.startswith("crates/nikaia/src/") and path.endswith(".rs"):
            rust += code_lines(git("cat-file", "-p", blob))
        elif path.endswith(".nika") and "/tools/" in path and path.rsplit("/", 1)[1] in COMPILER_NIKA:
            nika += code_lines(git("cat-file", "-p", blob))
    return rust, nika


def ts(s):
    return datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()


def agents(directory):
    rows = []
    for f in sorted(Path(directory).glob("agent-*.jsonl")):
        stamps, turns, ctx, out, first = [], 0, 0, 0, ""
        for l in f.open():
            d = json.loads(l)
            if "timestamp" in d:
                stamps.append(ts(d["timestamp"]))
            if d["type"] == "user" and not first:
                m = d["message"]["content"]
                first = m if isinstance(m, str) else ""
            if d["type"] == "assistant":
                u = d["message"].get("usage", {})
                turns += 1
                ctx += u.get("input_tokens", 0) + u.get("cache_read_input_tokens", 0) + u.get("cache_creation_input_tokens", 0)
                out += u.get("output_tokens", 0)
        if not stamps:
            continue
        active = sum(min(b - a, GAP) for a, b in zip(stamps, stamps[1:]))
        head = first[:500]
        if "READ-ONLY" in head:
            module = "plan"
        else:
            i, j = head.find("emit"), head.find("check")
            module = "plan" if i < 0 and j < 0 else "emit" if j < 0 or (0 <= i < j) else "check"
        rows.append(dict(id=f.stem[6:12], module=module, start=min(stamps), end=max(stamps),
                         turns=turns, ctx=ctx, out=out, minutes=active / 60, task=re.sub(r"\s+", " ", first)[:50]))
    return rows


def main():
    directory = sys.argv[1]
    since = sys.argv[2] if len(sys.argv) > 2 else "d927f6e~1"
    commits = []
    for l in git("log", "--reverse", "--format=%h %ct %s", f"{since}..origin/develop").splitlines():
        h, t, s = l.split(" ", 2)
        if re.search(r"\(0\.0\.\d+", s):
            commits.append((h, int(t), s))
    base_r, base_n = counts(since)
    prev = (base_r, base_n)
    print("### Progress per commit (code lines, `scripts/self_hosting.py` rule)\n")
    print("| commit | version | module | Rust | Nikaia | share | moved (Rust gone) |")
    print("| :--- | :--- | :--- | ---: | ---: | ---: | ---: |")
    moved = []
    for h, t, s in commits:
        r, n = counts(h)
        v = re.search(r"0\.0\.\d+", s).group(0)
        mod = "emit" if s.startswith("emit") else "check" if s.startswith("check") else "other"
        print(f"| {h} | {v} | {mod} | {r} | {n} | {100 * n / (r + n):.1f} % | {prev[0] - r} |")
        moved.append((t, mod, prev[0] - r, n - prev[1]))
        prev = (r, n)
    print("\n### Agents\n")
    print("| agent | module | turns | context tokens read (M) | written (k) | active min | Rust lines moved in its window | tokens per moved line |")
    print("| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |")
    for a in agents(directory):
        if a["module"] == "plan":
            gone = 0
        else:
            gone = sum(m for t, mod, m, _ in moved if mod == a["module"] and a["start"] <= t <= a["end"] + 60)
        per = f"{a['ctx'] / gone:,.0f}" if gone > 0 else "-"
        print(f"| {a['id']} | {a['module']} | {a['turns']} | {a['ctx'] / 1e6:.1f} | {a['out'] / 1e3:.1f} | {a['minutes']:.0f} | {gone} | {per} |")


if __name__ == "__main__":
    main()
