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

## Current score (2026-10-25, after 80f767a3)

| Ticket | S | O (M) | C (M) | Order |
| :--- | :--- | :--- | :--- | :--- |
| #837 | done 02c9b5f5 | | | |
| #753 --cpu dropped under RUSTFLAGS (owner: native builds) | 3 | 1 | 4 | 1 |
| #780 ByteSet in a variable, 115 vs 32 a row at v3 | 3 | 0.5 | 10 | 2 |
| #838 | 2 | 0.5 | 3 | 3 |
| #744 let over field/literal arms, rustc refuses | 3 | 1 | 8 | 4 |
| #582 map through a function value, E0308 | 3 | 0.5 | 10 | 5 |
| #747 step 1 NK1104 after NK1131 | 2 | 0.3 | 3 | 6 |
| E10-4 adapter switch (emit_function.nika built, not called from Rust) | 2 | 2 | 15 | 9 |
| #778/#775 audit (look built, not closed) | 1 | 0.5 | 3 | 7 |
| optional native compiler build + CI CPU count (owner) | 1 | ? | 5 | 8 |
| #825 | built (3573f202, 4b13ffa7), closed by hand | | | |
