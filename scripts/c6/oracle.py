#!/usr/bin/env python3
"""C-6 oracle: the findings and the lowered Rust of two compiler binaries over
one corpus must be byte-identical (findings text and order, fresh ids, facts).
usage: oracle.py OLD_BINARY NEW_BINARY REPO [WORKDIR]
Corpus: tests/**/*.nika and examples/**/*.nika, 142 files, about 35 s
(one file each, `nikaia lower --no-cache`, copied to one fixed path)."""
import glob, os, shutil, subprocess, sys, tempfile

old, new, repo = sys.argv[1:4]
work = sys.argv[4] if len(sys.argv) > 4 else tempfile.mkdtemp(prefix="c6-oracle-")
files = sorted(glob.glob(f"{repo}/tests/**/*.nika", recursive=True) + glob.glob(f"{repo}/examples/**/*.nika", recursive=True))

def run(binary, tag):
    out = {}
    d = os.path.join(work, "corpus")  # the same path for both, so messages match
    for f in files:
        shutil.rmtree(d, ignore_errors=True); os.makedirs(d)
        name = os.path.relpath(f, repo).replace("/", "__")
        target = os.path.join(d, name)
        shutil.copy(f, target)
        p = subprocess.run([binary, "lower", target, "--no-cache"], capture_output=True, timeout=120)
        rs = target[:-5] + ".rs"
        lowered = open(rs, "rb").read() if os.path.exists(rs) else b""
        out[name] = (p.returncode, p.stdout + p.stderr, lowered)
    return out

a = run(old, "old"); b = run(new, "new")
bad = [n for n in a if a[n] != b[n]]
for n in bad[:20]:
    which = [k for k, i in (("exit", 0), ("diagnostics", 1), ("rust", 2)) if a[n][i] != b[n][i]]
    print("DIFFERS", n, which)
print(f"{len(files)} files, {len(bad)} differ")
shutil.rmtree(work, ignore_errors=True)
sys.exit(1 if bad else 0)
