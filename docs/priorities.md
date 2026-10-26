# How the architect chooses the next topic

Model from an outside adviser (owner, 2026-10-25), made usable here.

    Priority = (strategic value + operational value) / total cost

All terms over the same horizon (the next about 20 builder runs), in tokens or
counts, never seconds.

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

The score decides, across areas. The default prior is the adviser's order:
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

## Current score (2026-10-25, after a2f40f98)

Figures that decide: the last four `check` moves cost 8.3-14.9 k tokens per
moved Rust line (`docs/tokenspeed.md`, target 50 k), so a slice of 1000 lines
costs about 10-15 M; E11-4a moved 1212 lines in one run. Rust left in
`crates/nikaia/src`: 25 555 lines (75.9 % of the toolchain is Nikaia);
`emit/mod.rs` 4605, `check/mod.rs` 1309. Per ticket a compiler bug still
scores higher (S 3 at C 3 M against S 2 at C 15 M), so self-hosting does **not**
go before bugs; it runs in its own lane beside the bug lane and never waits
(CLAUDE.md default: one bug builder, one self-hosting builder). The pause rule
does not fire: the last slices found one defect each and fixed it in the run.

| Ticket | S | O (M) | C (M) | Score | Lane, order |
| :--- | :--- | :--- | :--- | :--- | :--- |
| #851 `.trim().to_string()` stays `ref String` (hit in a tool) | 3 | 0.5 | 3 | 10.2 | bugs 1 |
| #856 literal owned where text of its own is needed (D31; blocks #853) | 3 | 0.5 | 6 | 5.1 | bugs 2 |
| #853 NK1301 literal `.to_string()` (owner), blocked by #856 | 2 | 0.3 | 4 | 5.1 | bugs 3 |
| #848 `m.get(k) ?? (tuple)` with text | 3 | 0.3 | 5 | 6.1 | bugs 3 |
| #857 E11-4b part 2, `a_stmt` (about 950; part 1 d1fe8d04 moved 290) | 2 | 1 | 12 | 1.8 | self-hosting 1 |
| #854 E11-4b statements and blocks (`emit/mod.rs` 2222-3720, about 1500) | 2 | 1 | 15 | 1.4 | self-hosting 1 |
| #855 E11-4c host callbacks go + E11-5 template tables, `view_slots` (about 450) | 2 | 1 | 6 | 3.5 | self-hosting 2 (after #854) |
| C-7 rest, `check/mod.rs` (#584): research brief first | 2 | 1 | 15 | 1.4 | self-hosting 3 |
| `emit/seeds.rs` (951) | 2 | 0.5 | 10 | 2.1 | self-hosting 4 |
