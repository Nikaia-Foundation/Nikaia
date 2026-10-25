# How we build Nikaia

We keep changing the way we work, and each change is a row below. The table says what we did, what we hoped for, and what was measured; where nobody counted, it says so.

| Iteration | Date | Action taken | Hope | Measured |
| :--- | :--- | :--- | :--- | :--- |
| 1 | 2026-10-11 | Self-hosting by long agents without a turn limit | Move Rust into Nikaia fast | Share 43.8 % to 49.8 % in 24 commits (0.0.787-0.0.810); 120-190k context tokens per moved line; agents of 695 and 768 turns ended at 566-626k context, 71-78 % of the cost after 300k |
| 2 | 2026-10-11 | Short Opus planning agents decide what builders were stuck on | Unblock builders cheaply | 7-21 M tokens each against 219-276 M for a long builder; value counted in builders unblocked, not in lines |
| 3 | 2026-10-11 | One agent, one slice, restart at about 120 turns | Computed: about one third of the cost | 71k against 143k tokens per moved line (11 fresh builders, 205 M tokens, 2,903 net lines; two long agents, 495 M, 3,459 lines); target below 50k; share 54.3 % at 0.0.823. Two cohorts, not yet a trend |
| 4 | 2026-10-11 | `rs2nika` draft as the default start of a move | Builder starts from a draft, not an empty file | About one turn saved per unit (C-5b); the twin file in the other module saved more |
| 5 | 2026-10-11 | Research brief (at most 80 lines) before the builder | Builder reads the brief instead of the plan, graph and issue | Not measured |
| 6 | 2026-10-11 | Set-up of models: Sonnet medium and Opus medium tried; agents now Opus low, main session Sonnet 5.5 low with a 200,000-token window | Owner read that Opus low beats Sonnet medium; the window keeps the session under 300k | Not measured; no model or effort choice was benchmarked |
| 7 | 2026-10-17 | Ten build agents measured | Find where the tokens go | 52k tokens at start, 35-40k of it descriptions of 41 unused tools; growth about 1k per turn; 7 of 10 ran past 200 turns; 10-20 % of cost spent polling |
| 8 | 2026-10-17 | `builder` agent type with six tools; waiting in one call | Start context from 52k to about 15k; no polling cost | Start context about 15k (the size CLAUDE.md gives); Not re-measured on a run |
| 9 | 2026-10-21 | `--profile fast` (#724): release optimisation, incremental | Shorter build after a one-file change | Stated in CLAUDE.md: about 18 s instead of 70 s for the binary; no count taken |
| 10 | 2026-10-24 | Rule: count, never time; callgrind for performance | Figures that survive a change of machine | The same binary moved 1.4-1.9 times between days; counts do not |
| 11 | 2026-10-24 | Architect role (Opus, few tools, GitHub issues), main session as carrier, stage 1: one builder at a time | One agent reads the issues and returns one work order | First run row: 38 turns, 1.2 M tokens, no line moved. Not enough to judge |
| 12 | 2026-10-24 | Per-file cache keys for the lowering (#667) | No stale or needless re-lowering | Built; a review found a stale-hit risk in the first form and filed it as #801 before harm |
| 13 | 2026-10-24 | `lower-std`: one Rust module per `.nika` (0.0.1207) | A one-file change recompiles one file | 1 of 256 objects changed on a one-file change; was one 7.4 MB unit |
| 14 | 2026-10-24 | Test binaries 267 into one (`--test it`) | Fewer links and less disk per change | 267 binaries of 16-20 MB become 1; saving in links read from the tree, disk not re-measured |
| 15 | 2026-10-24 | CI: tests built in a step of their own (#795) | Compiling separated from running | Not measured; one slip (YAML error) fixed in 8d56f29c |
| 16 | 2026-10-24 | Reading three CI runs (`docs/ci-cache.md`) | Decide whether a CI cache pays | Up to 40 % of a run is queue wait; Test is 68-83 % of a job; the owner parked the cache plan |
| 17 | 2026-10-24 | Long jobs: CI is the verify service (`docs/long-jobs.md`) | Builders stop running the long jobs | Touched tests about 10 min, full local suite about 45 min before the single binary; no counts |
| 18 | 2026-10-24 | Some(x) and None over a `T?` refused (ADR-352 D5, NK2704, NK2705); the tools write x and null | One way to write an optional, not two | Not measured |
| 19 | 2026-10-25 | Architect chooses the topic and writes one brief per builder to a file; the supervisor starts the builder with only the path | Builder context holds only what the brief names; fewer turns | Experiment running; not measured |
| 20 | 2026-10-25 | Limits on the disk cache (#826; brief sets `NIKAIA_CACHE_LIMIT=2G`, own `target/` deleted, at most two clones with `target/`) | The shared small disk stops running out | Not measured |
| 21 | 2026-10-25 | Pilot: QA agent (Sonnet low, runs the suite like CI) and review agent (Opus medium) | Red `develop` and defects found by a role whose duty it is | Pilot; not measured |
