# How the architect chooses the next topic

Model from an outside adviser (owner, 2026-10-25), made usable here.

    Priority = (strategic value + operational value) / total cost

All terms over the same horizon (the next about 20 builder runs), in tokens or
counts, never seconds.

**Owner's direction (2026-10-27): finish self-hosting.** Until
`crates/nikaia/src` holds no compiler logic in Rust, self-hosting goes first:
the main lane (`docs/briefs/`) holds moves only, at least two a batch, the
largest Rust files first (`project.rs`, `parser/mod.rs`, `comptime_run.rs`,
`sysroot.rs`, `emit/mod.rs`, `modules.rs`, `contracts/mod.rs`,
`check/mod.rs`); its one exception is a fix-forward for a red `develop`. A
compiler bug a move finds goes as a brief into the bug lane
(`docs/briefs/bugs/batch.txt`, with a `Blocks:` line) and the move waits for
its fix there (owner, 2026-10-31; `docs/architect.md`, *Lanes*). The bug lane
orders the other defects by the score below. The pause rule (more bugs found than fixed) still stops the self-hosting lane.
Each move starts from an `rs2nika` draft; a converter defect met on the way
is fixed in the converter (the 60 % rule in `scripts/drive.nika`). A move
is faithful by default; a schematic reshape is taught to the converter first,
a real redesign needs a named reason and gets no draft (`docs/architect.md`,
*Briefs and reports*, owner 2026-10-29).

## Estimating the terms per ticket

| Term | Estimate from | Unit |
| :--- | :--- | :--- |
| S, strategic | open compiler bugs closed or unblocked (issues labelled `ready` it fixes or frees); self-hosting lines moved (`scripts/self_hosting.py`), weighted by the *Rust* lines replaced, not by Nikaia tokens (Nikaia is shorter) | 0-3 |
| O, operational | tokens saved later: builder runs it unblocks x tokens a workaround or stall costs (`docs/tokenspeed.md`); CI instructions or minutes of jobs removed, as counts; turns per build cycle saved | M tokens |
| C, cost | tokens of comparable past runs (`docs/tokenspeed.md`, about 50-150 k per moved line, 5-30 M per fix); + test/integration; + maintenance (new code to keep); + opportunity: what self-hosting or bug work waits meanwhile | M tokens |

S is a coarse grade: 3 = fixes a defect a builder hit or a wrong value;
2 = moves a slice or closes a refusal; 1 = completeness. Score = (S x 10 M + O) / C;
only the order matters, not the number.

## Order

The score decides within each lane (the lanes are set above). The default prior is the adviser's order:
compiler bugs, self-hosting, CI speed, `rs2nika` (the converter only when it
measurably speeds a planned move, not for completeness). A bug a builder is
blocked on comes first. Any other departure from bugs-first is allowed when
the figures say so, and the brief states them. The owner delegated this rule
and the weight (10M per S point) to the architect (2026-10-25); the architect
revises the weight from measurements.

## Before scoring a ticket

Check that it is still open in the code: `git log --oneline -i --grep='#N'
origin/develop`. A commit that names the issue with `(#N)` or `Part of #N`
may have built it without closing it (#800, #801: both built, briefed again).
Such an issue is closed by hand with the commit named, not briefed.

## In each brief

Two or three lines: `S=<grade> (why), O=<M tokens> (why), C=<M tokens> (from
which past run), area`; on a departure from bugs-first, the figures.

## Current figures

In `docs/architect.md`, *State*. Past scores are in git (`git log -p docs/priorities.md`).
