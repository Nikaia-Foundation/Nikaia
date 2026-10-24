#!/usr/bin/env python3
"""Ledger of what the self-hosting agents cost against what they moved.

Two sources: the agents' transcripts (tokens, turns, active time) and the git
history of `develop` (Rust lines gone, Nikaia lines added, per commit). A
line is counted as `scripts/self_hosting.py` counts it: not blank, not only a
comment. Counts, never wall-clock claims about the machine: the minutes are
active minutes of the agent (gaps over 10 minutes, a build waiting, are cut).

Usage:
    scripts/tokenspeed.py AGENTS_DIR [SINCE_COMMIT]   print the two tables
    scripts/tokenspeed.py --short AGENTS_DIR          one Runs row for the latest agent

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
    for f in sorted([*Path(directory).glob("agent-*.jsonl"), *Path(directory).glob("*.output")]):
        stamps, turns, ctx, out, first = [], 0, 0, 0, ""
        late300 = late150 = 0
        files, adds = set(), []
        for l in f.open():
            try:
                d = json.loads(l)
            except json.JSONDecodeError:
                continue  # a line cut off while the agent ran
            if "timestamp" in d:
                stamps.append(ts(d["timestamp"]))
            if d["type"] == "user" and not first:
                m = d["message"]["content"]
                first = m if isinstance(m, str) else ""
            if d["type"] == "assistant":
                u = d["message"].get("usage", {})
                turns += 1
                t = u.get("input_tokens", 0) + u.get("cache_read_input_tokens", 0) + u.get("cache_creation_input_tokens", 0)
                ctx += t
                late300 += t if t > 300_000 else 0
                late150 += t if t > 150_000 else 0
                for b in d["message"].get("content", []):
                    if isinstance(b, dict) and b.get("type") == "tool_use":
                        i = b.get("input", {})
                        if b["name"] in ("Edit", "Write") and "file_path" in i:
                            files.add(i["file_path"].replace(str(ROOT) + "/", ""))
                        elif b["name"] == "Bash" and "git add" in i.get("command", ""):
                            adds.append(i["command"])
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
        rows.append(dict(id=f.stem.removeprefix("agent-")[:6], module=module, start=min(stamps), end=max(stamps),
                         turns=turns, ctx=ctx, late300=late300, late150=late150,
                         files=files | {w for c in adds for w in re.findall(r"[\w./-]+\.\w+", c.split("git add", 1)[1].split("&&")[0])},
                         full=f.stem.removeprefix("agent-"), out=out, minutes=active / 60, task=re.sub(r"\s+", " ", first)[:50]))
    return rows


def progress(since):
    """(commit, version, module, rust, nikaia, rust gone, time) per versioned commit."""
    commits = []
    for l in git("log", "--reverse", "--format=%h %ct %s", f"{since}..origin/develop").splitlines():
        h, t, s = l.split(" ", 2)
        if re.search(r"\(0\.0\.\d+", s):
            commits.append((h, int(t), s))
    rows, prev = [], counts(since)[0]
    for h, t, s in commits:
        r, n = counts(h)
        v = re.search(r"0\.0\.\d+", s).group(0)
        mod = "emit" if s.startswith("emit") else "check" if s.startswith("check") else "other"
        rows.append((h, v, mod, r, n, prev - r, t))
        prev = r
    return rows


def commit_gone(h):
    """Rust code lines gone (net) in crates/nikaia/src by one commit, and its changed files."""
    names = git("show", "--format=", "--name-only", h).split()
    gone = 0
    for n in names:
        if n.startswith("crates/nikaia/src/") and n.endswith(".rs"):
            gone += code_lines(git("show", f"{h}~1:{n}")) - code_lines(git("show", f"{h}:{n}"))
    return names, gone


def gone_in(a, rows=None):
    """Rust lines moved by the commits attributed to the agent, and how.

    A commit in the agent's window (start .. end + 60 s) belongs to it when it
    changed a file the agent edited (Edit/Write or `git add` in its transcript)
    or its message names the agent id. Commits with no versioned subject count too."""
    if a["module"] == "plan":
        return 0
    total = 0
    for l in git("log", "--format=%h %ct %s", f"--since={int(a['start'])}", f"--until={int(a['end']) + 60}", "origin/develop").splitlines():
        h, _, subj = l.split(" ", 2)
        names, g = commit_gone(h)
        if a["full"][:6] in subj or any(n in a["files"] for n in names):
            total += g
    return total


ATTRIBUTION = ("Attribution: a commit on `origin/develop` inside the agent's window belongs to the agent when it "
               "changed a file the agent edited (Edit/Write/`git add` in its transcript) or its subject names the agent id; "
               "moved = Rust code lines gone net from crates/nikaia/src by those commits. "
               "Late share = part of the context read in turns whose context was over 150k / 300k tokens.")


def short(directory):
    a = max(agents(directory), key=lambda a: a["end"])
    since = git("rev-list", "-1", f"--before={int(a['start'])}", "origin/develop").strip()
    gone = gone_in(a)
    per = f"{a['ctx'] / gone:,.0f}" if gone > 0 else "-"
    day = datetime.fromtimestamp(a["end"]).strftime("%Y-%m-%d")
    print(f"| {day} | {a['id']} | {a['module']} | {a['turns']} | {a['ctx'] / 1e6:.1f} | {gone} | {per} | |")


def main():
    if sys.argv[1] == "--short":
        return short(sys.argv[2])
    directory = sys.argv[1]
    since = sys.argv[2] if len(sys.argv) > 2 else "d927f6e~1"
    rows = progress(since)
    print("### Progress per commit (code lines, `scripts/self_hosting.py` rule)\n")
    print("| commit | version | module | Rust | Nikaia | share | moved (Rust gone) |")
    print("| :--- | :--- | :--- | ---: | ---: | ---: | ---: |")
    for h, v, mod, r, n, g, _ in rows:
        print(f"| {h} | {v} | {mod} | {r} | {n} | {100 * n / (r + n):.1f} % | {g} |")
    print("\n### Agents\n")
    print(ATTRIBUTION + "\n")
    print("| agent | module | turns | context tokens read (M) | written (k) | active min | Rust lines moved (attributed) | tokens per moved line | read after 150k | read after 300k |")
    print("| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
    for a in agents(directory):
        gone = gone_in(a)
        per = f"{a['ctx'] / gone:,.0f}" if gone > 0 else "-"
        l1, l3 = (f"{100 * a[k] / a['ctx']:.0f} %" if a["ctx"] else "-" for k in ("late150", "late300"))
        print(f"| {a['id']} | {a['module']} | {a['turns']} | {a['ctx'] / 1e6:.1f} | {a['out'] / 1e3:.1f} | {a['minutes']:.0f} | {gone} | {per} | {l1} | {l3} |")
    rs = agents(directory)
    tot = sum(a["ctx"] for a in rs)
    over = sum(a["ctx"] for a in rs if a["turns"] > 120)
    print(f"\n{len(rs)} agents, {tot / 1e6:.0f} M read, agents over 120 turns = {100 * over / tot:.0f} % of reads.")


if __name__ == "__main__":
    main()
