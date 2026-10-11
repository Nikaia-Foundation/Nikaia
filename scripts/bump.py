#!/usr/bin/env python3
"""Raise the version, and repair it after a rebase (CLAUDE.md, "A change to the
language raises the version by one").

What one version raise touches, and what this writes:
  * `**Version:** X (Draft)` and `**Date:** D` in the three specification parts;
  * a `## [X] — D` heading with its title and entries on top of `CHANGELOG.md`;
  * the `(X)` of the rows of `scripts/self_hosting.py` for new `.nika` files;
  * `assets/badges/*.svg`, through `scripts/badges.py`.

Usage:
  scripts/bump.py new --title "check: ... (#558, #125)" --entry "what it changed" [--entry ...]
                      [--nika check_foo.nika="check: what it holds"] [--version X] [--date D]
      Before the commit. The version is one above the highest of: the
      specification's line, the top CHANGELOG heading, and origin/develop's
      line (after `git fetch`). Prints the version, for the commit subject.

  scripts/bump.py rebase-fix [--upstream REF]
      After `git pull --rebase origin develop`, stopped on a conflict or not.
      Takes upstream's version files, puts this branch's CHANGELOG headings on
      top with the next free numbers, rewrites the version and date lines and
      this branch's self_hosting rows to match, redraws the badges and
      `git add`s what it resolved. Conflicts that are not only version/date
      lines are left for you and listed. Then: `git rebase --continue`, and
      reword the commit subject if it names the old version (printed).
      REF defaults to HEAD during a rebase, origin/develop otherwise.

  scripts/bump.py check
      Exit 1 unless the three version lines agree with each other and with the
      top CHANGELOG heading, the top headings are unique and descending, and
      the badges are current.
"""

import argparse
import datetime
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(subprocess.run(['git', 'rev-parse', '--show-toplevel'], capture_output=True, text=True).stdout.strip() or '.')
SPEC = [ROOT / 'docs' / 'specification' / f for f in ('10-nikaia-light.md', '20-nikaia-advance.md', '30-nikaia-tooling.md')]
CHANGELOG = ROOT / 'CHANGELOG.md'
SELF = ROOT / 'scripts' / 'self_hosting.py'
BADGES = ROOT / 'assets' / 'badges'
HEAD_RE = re.compile(r'^## \[(\d+)\.(\d+)\.(\d+)\]', re.M)
VER_RE = re.compile(r'^\*\*Version:\*\*\s*(\d+\.\d+\.\d+)', re.M)


def git(*a, check=True):
    r = subprocess.run(['git', '-C', str(ROOT), *a], capture_output=True, text=True)
    if check and r.returncode:
        sys.exit(f"git {' '.join(a)}: {r.stderr.strip()}")
    return r.stdout


def show(ref, path):
    r = subprocess.run(['git', '-C', str(ROOT), 'show', f'{ref}:{path.relative_to(ROOT)}'], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def vt(s):
    return tuple(int(x) for x in s.split('.'))


def vs(t):
    return '.'.join(str(x) for x in t)


def today():
    return datetime.datetime.now(datetime.timezone.utc).date().isoformat()


# ------------------------------------------------------------------ CHANGELOG

def split_changelog(text):
    """(intro, [(version_tuple, block_text)]) - a block is a heading and its body."""
    ms = list(HEAD_RE.finditer(text))
    if not ms:
        return text, []
    intro = text[:ms[0].start()]
    blocks = []
    for i, m in enumerate(ms):
        end = ms[i + 1].start() if i + 1 < len(ms) else len(text)
        blocks.append(((int(m[1]), int(m[2]), int(m[3])), text[m.start():end]))
    return intro, blocks


def body_of(block):
    """A block without its heading line: what identifies an entry across renumbering."""
    return block.split('\n', 1)[1].strip() if '\n' in block else ''


def renumber(block, new):
    return re.sub(r'^## \[\d+\.\d+\.\d+\]( — \S+)?', lambda m: f'## [{vs(new)}] — {today()}', block, count=1)


def top_version(text):
    _, b = split_changelog(text or '')
    return max((v for v, _ in b), default=(0, 0, 0))


def spec_version(text):
    m = VER_RE.search(text or '')
    return vt(m[1]) if m else (0, 0, 0)


# ------------------------------------------------------------------ conflicts

CONFLICT = re.compile(r'^<<<<<<< [^\n]*\n(.*?)^(?:\|\|\|\|\|\|\| [^\n]*\n.*?)?^=======\n(.*?)^>>>>>>> [^\n]*\n', re.M | re.S)


def resolve_blocks(text, decide):
    """Replace each conflict block by decide(ours, theirs) -> str or None (left as is)."""
    left = []

    def rep(m):
        r = decide(m[1], m[2])
        if r is None:
            left.append(m[0])
            return m[0]
        return r
    return CONFLICT.sub(rep, text), left


def only_version_lines(side):
    return all(re.match(r'^\*\*(Version|Date):\*\*', l) for l in side.splitlines() if l.strip())


def union_rows(ours, theirs):
    """self_hosting.py: both sides added or changed rows. Every key is kept; where
    both have a key, this branch's row (theirs, during a rebase) wins."""
    if not all(re.match(r'^\s*("[^"]+": ".*",|#.*)?\s*$', l) for l in (ours + theirs).splitlines()):
        return None
    key = lambda l: (re.match(r'^\s*"([^"]+)":', l) or [None, None])[1]
    mine = {key(l): l for l in theirs.splitlines(True) if key(l)}
    out = [mine.pop(key(l), l) if key(l) else l for l in ours.splitlines(True)]
    return ''.join(out) + ''.join(l for k, l in mine.items())


# ------------------------------------------------------------------ writers

def set_spec(version, date):
    for p in SPEC:
        s = p.read_text(encoding='utf-8')
        s, n = re.subn(r'^\*\*Version:\*\*\s*\S+', f'**Version:** {vs(version)}', s, count=1, flags=re.M)
        if n != 1:
            sys.exit(f'{p}: no **Version:** line')
        s = re.sub(r'^\*\*Date:\*\*\s*\S+', f'**Date:** {date}', s, count=1, flags=re.M)
        p.write_text(s, encoding='utf-8')


def set_rows(rows, version):
    """rows: {file.nika: description or None}; None keeps the description, sets (version)."""
    s = SELF.read_text(encoding='utf-8')
    for name, what in rows.items():
        line = re.search(r'^(\s*)"' + re.escape(name) + r'": "(.*)",\s*$', s, re.M)
        if line:
            text = what if what is not None else line[2]
            if re.search(r'\(\d+\.\d+\.\d+(, \d+\.\d+\.\d+)*\)$', text):
                text = re.sub(r'\d+\.\d+\.\d+\)$', f'{vs(version)})', text) if what is None else f'{text} ({vs(version)})'
            else:
                text = f'{text} ({vs(version)})'
            s = s[:line.start()] + f'{line[1]}"{name}": "{text}",' + s[line.end():]
        else:
            if what is None:
                continue
            end = s.index('\n}\n', s.index('COMPILER_NIKA = {'))
            s = s[:end] + f'\n    "{name}": "{what} ({vs(version)})",' + s[end:]
    SELF.write_text(s, encoding='utf-8')


def badges():
    r = subprocess.run([sys.executable, str(ROOT / 'scripts' / 'badges.py')], capture_output=True, text=True, cwd=ROOT)
    if r.returncode:
        sys.exit('scripts/badges.py: ' + r.stderr.strip())


def esc(s):
    return s.replace('\\n', '\n')


# ------------------------------------------------------------------ commands

def cmd_new(a):
    cl = CHANGELOG.read_text(encoding='utf-8')
    git('fetch', '-q', 'origin', 'develop', check=False)
    upstream = spec_version(show('origin/develop', SPEC[0]))
    current = max(spec_version(SPEC[0].read_text(encoding='utf-8')), top_version(cl), upstream)
    version = vt(a.version) if a.version else (current[0], current[1], current[2] + 1)
    date = a.date or today()
    intro, blocks = split_changelog(cl)
    entries = ''.join(f'- {esc(e)}\n' for e in a.entry)
    block = f'## [{vs(version)}] — {date}\n\n**{a.title}**\n{entries}\n'
    CHANGELOG.write_text(intro + block + ''.join(b for _, b in blocks), encoding='utf-8')
    set_spec(version, date)
    rows = {}
    for n in a.nika:
        name, _, what = n.partition('=')
        rows[name.strip()] = what.strip() or None
    if rows:
        set_rows(rows, version)
    badges()
    print(vs(version))


def cmd_rebase_fix(a):
    in_rebase = subprocess.run(['git', '-C', str(ROOT), 'rev-parse', '-q', '--verify', 'REBASE_HEAD'], capture_output=True).returncode == 0
    upstream = a.upstream or ('HEAD' if in_rebase else 'origin/develop')
    up_cl = show(upstream, CHANGELOG) or ''
    up_intro, up_blocks = split_changelog(up_cl)
    up_bodies = {body_of(b) for _, b in up_blocks}
    # this branch's entries: during a rebase, the ones the commit being applied adds
    if in_rebase:
        before = {body_of(b) for _, b in split_changelog(show('REBASE_HEAD^', CHANGELOG) or '')[1]}
        mine = [(v, b) for v, b in split_changelog(show('REBASE_HEAD', CHANGELOG) or '')[1] if body_of(b) not in before]
    else:
        text = CHANGELOG.read_text(encoding='utf-8')
        if '<<<<<<<' in text:
            sys.exit('CHANGELOG.md has conflict markers but no rebase is in progress: resolve by hand')
        mine = [(v, b) for v, b in split_changelog(text)[1] if body_of(b) not in up_bodies]
    mine = [(v, b) for v, b in mine if body_of(b) not in up_bodies]
    base = max((v for v, _ in up_blocks), default=(0, 0, 0))
    base = max(base, spec_version(show(upstream, SPEC[0])))
    mine.sort(key=lambda vb: vb[0])
    renum = {}
    new_blocks = []
    nxt = base
    for v, b in mine:
        nxt = (nxt[0], nxt[1], nxt[2] + 1)
        renum[v] = nxt
        new_blocks.append(renumber(b, nxt))
    top = nxt
    CHANGELOG.write_text(up_intro + ''.join(reversed(new_blocks)) + ''.join(b for _, b in up_blocks), encoding='utf-8')
    resolved, left = [CHANGELOG], []
    # specification: conflicts of version/date lines take upstream's side; the line is rewritten below
    for p in SPEC:
        s = p.read_text(encoding='utf-8')
        if '<<<<<<<' in s:
            s, l = resolve_blocks(s, lambda o, t: o if only_version_lines(o) and only_version_lines(t) else None)
            p.write_text(s, encoding='utf-8')
            if l:
                left.append(p)
                continue
        resolved.append(p)
    if top != base:
        set_spec(top, today())
    # self_hosting.py: a conflict of two added rows keeps both
    s = SELF.read_text(encoding='utf-8')
    if '<<<<<<<' in s:
        s, l = resolve_blocks(s, union_rows)
        SELF.write_text(s, encoding='utf-8')
        if l:
            left.append(SELF)
    if SELF not in left:
        resolved.append(SELF)
        up = show(upstream, SELF) or ''
        for old, new in sorted(renum.items(), reverse=True):
            if old == new:
                continue
            names = [n for n in re.findall(r'^\s*"([^"]+)": ".*\(' + re.escape(vs(old)) + r'\)",\s*$', s, re.M)
                     if not re.search(r'^\s*"' + re.escape(n) + r'": "' + re.escape(re.search(r'^\s*"' + re.escape(n) + r'": "(.*)",', s, re.M)[1]) + r'",', up, re.M)]
            if names:
                set_rows({n: None for n in names}, new)
                s = SELF.read_text(encoding='utf-8')
    if left:
        print('left for you (conflicts beyond the version lines):')
        for p in left:
            print('  ', p.relative_to(ROOT))
    badges()
    resolved.append(BADGES)
    git('add', *[str(p) for p in resolved])
    for old, new in sorted(renum.items()):
        print(f'{vs(old)} -> {vs(new)}' if old != new else f'{vs(new)} kept')
    subj = git('log', '-1', '--format=%s', 'REBASE_HEAD' if in_rebase else 'HEAD').strip()
    for old, new in renum.items():
        if old != new and vs(old) in subj:
            print(f'reword the commit: {subj.replace(vs(old), vs(new))}')
    if in_rebase and not left:
        print('then: git rebase --continue')


def cmd_check(_):
    bad = []
    vers = {p.name: spec_version(p.read_text(encoding='utf-8')) for p in SPEC}
    if len(set(vers.values())) != 1:
        bad.append(f'version lines differ: {vers}')
    cl = CHANGELOG.read_text(encoding='utf-8')
    _, blocks = split_changelog(cl)
    top = [v for v, _ in blocks[:30]]
    if top and top[0] != list(vers.values())[0]:
        bad.append(f'top CHANGELOG heading {vs(top[0])} != specification {vs(list(vers.values())[0])}')
    for i in range(len(top) - 1):
        if top[i] <= top[i + 1]:
            bad.append(f'CHANGELOG headings not descending: {vs(top[i])} then {vs(top[i + 1])}')
    for p in SPEC + [CHANGELOG, SELF]:
        if '<<<<<<<' in p.read_text(encoding='utf-8'):
            bad.append(f'conflict markers in {p.relative_to(ROOT)}')
    r = subprocess.run([sys.executable, str(ROOT / 'scripts' / 'badges.py'), '--check'], capture_output=True, text=True, cwd=ROOT)
    if r.returncode:
        bad.append(r.stderr.strip() or r.stdout.strip())
    for b in bad:
        print(b)
    sys.exit(1 if bad else 0)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest='cmd', required=True)
    n = sub.add_parser('new')
    n.add_argument('--title', required=True)
    n.add_argument('--entry', action='append', default=[])
    n.add_argument('--nika', action='append', default=[], help='file.nika="description" (adds or renumbers its row)')
    n.add_argument('--version')
    n.add_argument('--date')
    r = sub.add_parser('rebase-fix')
    r.add_argument('--upstream')
    sub.add_parser('check')
    a = ap.parse_args()
    {'new': cmd_new, 'rebase-fix': cmd_rebase_fix, 'check': cmd_check}[a.cmd](a)


if __name__ == '__main__':
    main()
