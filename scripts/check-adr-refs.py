#!/usr/bin/env python3
"""Every `ADR-0NN Dn` / `ADR-topic-n Dn` / `… §x` citation must point at a heading
that exists, and every ADR name belongs to exactly one record.

A record is named `ADR-<topic>-<n>`: one to three lowercase words joined by `-`
and a number (ADR-decision-records-1). The records from before carry a bare
three-digit number until a consolidation replaces them; both are checked here.
**Python, not Nikaia, for now** (ADR-decision-records-1 D5): the move waits
for `std::regex` (#673 step 1), which this tool is made of.

A decision is only citable if the number is stable, so a dangling `D7` is a
defect in the same way a dangling link is. Run from the repository root; exits
non-zero and lists what is dangling.

**The second check is here because the first one missed a collision.** Two
sessions claimed 074 on the same afternoon and the second overwrote the first
record's file - every citation still resolved, because the file was still
there and still had a `D1`. What the repository did notice, and nobody read,
was the index: it carried **two rows for 074**, one per record. So that is what
is checked - one index row per number, one file per number, and the file's own
title agreeing with its name. A claim in the reserved table is a promise between
people; this is the part a machine can keep.
"""

import glob
import os
import re
import sys

# A record's name after `ADR-`: a number of the old scheme, or a topic of one to
# three lowercase words and a number (ADR-decision-records-1 D1).
TOPIC = r"[a-z][a-z0-9]*(?:-[a-z][a-z0-9]*){0,2}-[1-9][0-9]*"
NAME = r"(?:\d{3}|" + TOPIC + r")"
FILE = re.compile(r"adr-(" + NAME + r")\.md$")


def name_of(path):
    """The record's name from its file, or None for a file that breaks the scheme."""
    m = FILE.search(path)
    return m.group(1) if m else None


def records(where):
    """The record files of a directory: every `adr-*.md` but the template."""
    return sorted(glob.glob(f"{where}/adr-*.md"))

ADR_DIR = "docs/specification/adr"
# Records a later record replaced. They are read for history only, and only
# `docs/history/` and `CHANGELOG.md` may cite them (`banished()`).
ARCHIVE = "docs/history/adr"

SEARCH = [
    "crates/**/*.rs",
    "crates/**/*.nika",
    "tests/**/*",
    "examples/**/*",
    "benches/**/*",
    "docs/**/*.md",
    "scripts/**/*",
    "README.md",
    "manifesto.md",
    "guide/**/*.md",
    "editors/**/*",
    "**/Cargo.toml",
    "rust-toolchain.toml",
    "index.html",
]


def collisions():
    """One record per number: one index row, one file, and a title that agrees.

    Reported rather than raised, so a run says everything that is wrong at once.
    """
    wrong = []
    index = open(f"{ADR_DIR}/README.md", encoding="utf-8").read()
    # An index row opens with `| [0NN](adr-0NN.md) | <prose> |`. Two other tables
    # in the same file open the same way and are **not** ownership: the reserved
    # table's rows hold a bare number and no link, and the amendments table's
    # second cell is another ADR rather than prose - which is what tells them
    # apart here.
    rows = [
        num
        for num, second in re.findall(
            r"^\|\s*\[(" + NAME + r")\]\(adr-\1\.md\)\s*\|([^|]*)\|", index, re.M
        )
        if not re.match(r"\s*\[" + NAME + r"\]\(adr-" + NAME + r"\.md\)", second)
    ]
    for num in sorted(set(rows)):
        if rows.count(num) > 1:
            wrong.append(f"ADR-{num}: {rows.count(num)} index rows")

    archive = open(f"{ARCHIVE}/README.md", encoding="utf-8").read()
    archived = re.findall(r"^\|\s*\[(" + NAME + r")\]\(adr-\1\.md\)\s*\|", archive, re.M)
    for num in sorted(set(rows) & set(archived)):
        wrong.append(f"ADR-{num}: indexed both as live and as archived")
    for path in records(ADR_DIR) + records(ARCHIVE):
        num = name_of(path)
        if num is None:
            wrong.append(
                f"{path}: not a record name - `adr-<topic>-<n>.md`, one to three"
                " lowercase words and a number"
            )
            continue
        listed = archived if path.startswith(ARCHIVE) else rows
        first = open(path, encoding="utf-8").readline()
        said = re.match(r"#\s*ADR-(" + NAME + r"):", first.strip())
        if not said:
            wrong.append(f"{path}: the first line does not open `# ADR-{num}: …`")
        elif said.group(1) != num:
            wrong.append(f"{path}: titled ADR-{said.group(1)}")
        if num not in listed:
            wrong.append(f"ADR-{num}: a record with no index row")
    live = {name_of(p) for p in records(ADR_DIR)} - {None}
    for num in sorted(live & ({name_of(p) for p in records(ARCHIVE)} - {None})):
        wrong.append(f"ADR-{num}: a file both live and archived")
    return wrong


def headings():
    """What each ADR actually offers: its `Dn` and its top-level section numbers."""
    have = {}
    for path in records(ADR_DIR) + records(ARCHIVE):
        num = name_of(path)
        if num is None:
            continue
        text = open(path, encoding="utf-8").read()
        ds = set(re.findall(r"^#{2,4}\s+(D\d+)\b", text, re.M))
        # `## 3. Consequences` and `### 4.2 — …` both name a section.
        secs = set(re.findall(r"^#{2,4}\s+(\d+(?:\.\d+)*)[.\s]", text, re.M))
        have[num] = (ds, secs)
    return have


def citations():
    """Every (adr, label) pair referenced anywhere, with the file that does it."""
    seen = []
    files = set()
    for pattern in SEARCH:
        files |= {f for f in glob.glob(pattern, recursive=True) if os.path.isfile(f)}
    for f in sorted(files):
        if os.path.abspath(f) == os.path.abspath(__file__):
            continue
        # The history is read as it was written; a section it cites may since
        # have left a record. Build output is not ours to check.
        # A brief may cite the decision it asks a builder to write (R0's
        # ADR-294 D21): it is a plan, checked once the decision exists.
        if f.startswith(("docs/history/", "docs/briefs/")) or "/target/" in f:
            continue
        try:
            text = open(f, encoding="utf-8", errors="ignore").read()
        except OSError:
            continue
        # Two spellings reach the same place: a bare `ADR-023 D7`, and the
        # markdown form `[ADR-023](adr-023.md) D7` that every ADR header uses.
        for pattern in (
            r"ADR[- ]?(\d{3})((?:[^A-Za-z0-9]{0,4}(?:D\d+|§[\d.]+))+)",
            r"ADR-(" + TOPIC + r")((?:[^A-Za-z0-9]{0,4}(?:D\d+|§[\d.]+))+)",
            r"\]\((?:[\w./-]*/)?adr-(" + NAME + r")\.md\)((?:[^A-Za-z0-9]{0,4}(?:D\d+|§[\d.]+))+)",
        ):
            for m in re.finditer(pattern, text):
                for label in re.findall(r"D\d+|§[\d.]+", m.group(2)):
                    seen.append((m.group(1), label, f))
    return seen


# An ADR may point at a note ("the survey is over there"). It may not cite a
# numbered section of one, because that is borrowing authority from a file
# `docs/README.md` says holds nothing normative. The difference is the whole
# line between "see the survey" and "per rule 6".
NOTES = re.compile(
    r"(?:docs/)?(?:\.\./\.\./)?"
    r"(staging-candidates|handoff|error-corpus|technical_notes|"
    r"toolchain_architecture|project_status_and_roadmap)\.md[^\n]{0,12}§[\d.]+"
)


def inversions():
    """ADRs citing a section of a notes file as if it were a rule."""
    found = []
    for path in records(ADR_DIR):
        for line_no, line in enumerate(open(path, encoding="utf-8"), 1):
            if NOTES.search(line):
                found.append((path, line_no, line.strip()[:90]))
    return found


def banished():
    """A live file citing an archived record.

    An archived record was replaced by one that states the same decisions in
    their current form. Pointing a reader - or a model - at it sends them to
    the route rather than the answer and costs a second read, so only the
    history may name it: `docs/history/` and `CHANGELOG.md`.
    """
    archived = {name_of(p) for p in records(ARCHIVE)} - {None}
    if not archived:
        return []
    names = "|".join(re.escape(n) for n in sorted(archived))
    pattern = re.compile(r"ADR[- ]?(" + names + r")(?![A-Za-z0-9_])|adr-(" + names + r")\.md")
    found = []
    files = set()
    for p in SEARCH:
        files |= {f for f in glob.glob(p, recursive=True) if os.path.isfile(f)}
    for f in sorted(files):
        if f.startswith("docs/history/") or f == "CHANGELOG.md" or "__pycache__" in f or "/target/" in f:
            continue
        if os.path.abspath(f) == os.path.abspath(__file__):
            continue
        for line_no, line in enumerate(open(f, encoding="utf-8", errors="ignore"), 1):
            m = pattern.search(line)
            if m:
                found.append((f, line_no, m.group(0)))
    return found


def unknown(have):
    """A name of the topic scheme that no record carries, cited with or without a `Dn`.

    The old numbers are checked only where a `Dn` or `§` follows; a topic name is
    checked wherever it stands, because a name that looks right is the easier one
    to get wrong (`ADR-regex-1` against `ADR-regexp-1`).
    """
    found = []
    files = set()
    for p in SEARCH:
        files |= {f for f in glob.glob(p, recursive=True) if os.path.isfile(f)}
    for f in sorted(files):
        if f.startswith("docs/history/") or "__pycache__" in f or "/target/" in f:
            continue
        if os.path.abspath(f) == os.path.abspath(__file__):
            continue
        for line_no, line in enumerate(open(f, encoding="utf-8", errors="ignore"), 1):
            for m in re.finditer(r"(?<![A-Za-z0-9_])ADR-(" + TOPIC + r")(?![A-Za-z0-9_])", line):
                if m.group(1) not in have:
                    found.append((f, line_no, m.group(0)))
    return found


def main():
    have = headings()
    bad = []
    for adr, label, where in citations():
        if adr not in have:
            bad.append((adr, label, where, "no such ADR"))
            continue
        ds, secs = have[adr]
        if label.startswith("D"):
            if label not in ds:
                bad.append((adr, label, where, "no such decision"))
        else:
            sec = label.lstrip("§").rstrip(".")
            if sec and sec not in secs:
                bad.append((adr, label, where, "no such section"))

    inverted = inversions()
    collided = collisions()
    stale = banished()
    missing = unknown(have)

    if bad:
        print(f"{len(bad)} dangling ADR citation(s):")
        for adr, label, where, why in sorted(set(bad)):
            print(f"  {where}: ADR-{adr} {label} — {why}")
    if inverted:
        print(f"{len(inverted)} ADR(s) citing a section of a notes file as authority:")
        for path, line_no, text in inverted:
            print(f"  {path}:{line_no}: {text}")
        print("  A note holds nothing normative (docs/README.md). Link the file,")
        print("  or make the argument in the ADR.")
    if collided:
        print(f"{len(collided)} ADR name(s) not owned by exactly one record:")
        for line in collided:
            print(f"  {line}")
        print("  A name is taken when it is claimed in the index's reserved table,")
        print("  and a record is written once. Two rows or a mismatched title means")
        print("  two records are wearing one name - move the later one.")
    if stale:
        print(f"{len(stale)} citation(s) of an archived ADR outside the history:")
        for f, line_no, what in stale:
            print(f"  {f}:{line_no}: {what}")
        print("  Cite the record that replaced it (docs/history/adr/README.md maps")
        print("  every archived decision to its live one).")
    if missing:
        print(f"{len(missing)} ADR name(s) that no record carries:")
        for f, line_no, what in missing:
            print(f"  {f}:{line_no}: {what}")
    if bad or inverted or collided or stale or missing:
        return 1

    print(f"all ADR citations resolve ({len(citations())} checked)")
    print("no ADR borrows authority from a note")
    print(f"every ADR name belongs to one record ({len(headings())} checked)")
    print("no live file cites an archived record")
    return 0


if __name__ == "__main__":
    sys.exit(main())
