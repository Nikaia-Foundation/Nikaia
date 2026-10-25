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

## Order and rules

1. Compiler bugs (CLAUDE.md *Bugs before moving*, owner's rule: this is a rule,
   not a weight; a bug a builder is blocked on comes first regardless of score).
2. Self-hosting.
3. CI speed.
4. `rs2nika`: only when it measurably speeds a planned move (report
   `kept N of M lines`), is needed for it, and saves more than it costs and
   delays; not for completeness.

Within each band the score orders the tickets; across bands the order holds,
except that a CI or rs2nika item whose O exceeds a band-1 item's whole cost may
be raised by the owner.

## In each brief

Two or three lines: `S=<grade> (why), O=<M tokens> (why), C=<M tokens> (from
which past run), band <n>`.
