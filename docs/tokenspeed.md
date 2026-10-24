# Token speed: what the self-hosting agents cost against what they moved

A ledger. Each snapshot is dated and appended; old ones are not edited. The
measure is **context tokens read per Rust code line that left the compiler**
(the line rule is `scripts/self_hosting.py`'s: not blank, not only a comment).
Not seconds of the machine: the minutes are an agent's *active* minutes, a
pause over 10 minutes (a build, a stop) is cut. See `docs/tokensaverstrategy.md`
for why the context read is the cost.

## How to add a snapshot

```
scripts/tokenspeed.py ~/.claude/projects/<project>/<session>/subagents <since-commit> > snapshot.md
```

Paste it under a new `## YYYY-MM-DD` heading and add the reading. The agent
transcripts are not in the repository; the numbers are copied here so they
outlive the container. Commits are matched to an agent by the module prefix of
the subject (`emit:`/`check:`) inside the agent's time window; a commit with no
prefix is `other` and belongs to nobody. A moved line is *net*: an agent that
adds Rust on the way (an adapter) is charged for it.

## 2026-10-11

Since `03dec80` (0.0.786). Share at the end: 49.8 % (`scripts/self_hosting.py`).

### Progress per commit (code lines, `scripts/self_hosting.py` rule)

| commit | version | module | Rust | Nikaia | share | moved (Rust gone) |
| :--- | :--- | :--- | ---: | ---: | ---: | ---: |
| 8291853 | 0.0.787 | check | 41080 | 32016 | 43.8 % | 184 |
| fc995cd | 0.0.788 | check | 40788 | 32213 | 44.1 % | 292 |
| 534ebb2 | 0.0.787 | emit | 40371 | 32788 | 44.8 % | 417 |
| 73f2e80 | 0.0.790 | emit | 40158 | 33095 | 45.2 % | 213 |
| 3bdc0ad | 0.0.791 | check | 39575 | 33714 | 46.0 % | 583 |
| ab31772 | 0.0.792 | emit | 39500 | 33868 | 46.2 % | 75 |
| f4c5913 | 0.0.792 | check | 39421 | 33943 | 46.3 % | 79 |
| 803d6eb | 0.0.794 | check | 39421 | 34201 | 46.5 % | 0 |
| 0bb1721 | 0.0.795 | emit | 39508 | 34201 | 46.4 % | -87 |
| fcffd80 | 0.0.796 | emit | 39478 | 34291 | 46.5 % | 30 |
| 00d59d7 | 0.0.797 | check | 39203 | 34294 | 46.7 % | 275 |
| 5928082 | 0.0.798 | check | 38845 | 34737 | 47.2 % | 358 |
| 78b99ef | 0.0.799 | check | 38508 | 35049 | 47.6 % | 337 |
| 509b36f | 0.0.800 | emit | 38684 | 35116 | 47.6 % | -176 |
| ba0c53e | 0.0.801 | emit | 38382 | 35526 | 48.1 % | 302 |
| 977343f | 0.0.801 | other | 38382 | 35526 | 48.1 % | 0 |
| 190d784 | 0.0.802 | emit | 38035 | 36059 | 48.7 % | 347 |
| b3bb1c9 | 0.0.803 | check | 37931 | 36238 | 48.9 % | 104 |
| 28912a2 | 0.0.804 | check | 37824 | 36373 | 49.0 % | 107 |
| d927f6e | 0.0.805 | check | 37851 | 36373 | 49.0 % | -27 |
| 8e50534 | 0.0.807 | emit | 37795 | 36628 | 49.2 % | 56 |
| 441977f | 0.0.808 | emit | 37805 | 36632 | 49.2 % | -10 |
| bc73562 | 0.0.809 | other | 37834 | 36644 | 49.2 % | -29 |
| 8060d63 | 0.0.810 | emit | 37499 | 37264 | 49.8 % | 335 |

### Agents

| agent | module | turns | context tokens read (M) | written (k) | active min | Rust lines moved in its window | tokens per moved line |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| a5fd67 | check | 22 | 1.4 | 0.3 | 9 | 0 | - |
| a99ad9 | check | 163 | 20.6 | 1.6 | 28 | 0 | - |
| aa737a | plan | 72 | 6.9 | 0.6 | 5 | 0 | - |
| ab62ea | emit | 695 | 219.1 | 6.5 | 315 | 1167 | 187,742 |
| ab8a58 | check | 768 | 276.2 | 6.9 | 281 | 2292 | 120,518 |
| abfc7f | emit | 164 | 24.1 | 1.5 | 36 | 335 | 72,039 |
| ac786f | plan | 124 | 16.0 | 2.1 | 10 | 0 | - |
| af7610 | plan | 159 | 20.9 | 1.5 | 12 | 0 | - |

### Reading

* **Progress.** 24 commits (0.0.787-0.0.810) took the share from about
  43.8 % to 49.8 %: about 3,700 Rust code lines gone, about 5,200 Nikaia lines
  added. Rust lines gone per commit: 100-600; the slices that only declare
  state (`CheckFacts`, `EmitState`) move 0 or go negative, and are the price of
  the slices after them.
* **The two long agents (ab62ea emit, ab8a58 check): 219 M and 276 M context
  tokens for 695 and 768 turns,** 190k and 120k context tokens per moved Rust
  line, 5 and 4.7 active hours.
* **The first short agents** (abfc7f emit E7 first slice, 164 turns, 24 M,
  36 active minutes: 72k per moved line; a5fd67, a method-lowering fix, 22
  turns, 1.4 M, 0 lines moved but a language change of 0.0.809) cost 72k per moved
  line against 120-190k for the long agents (40-60 % of it); one slice is too
  few to call a trend. a99ad9 (C-2) was still running when this was taken: its
  line shows 0 moved.
* **Planning is cheap** (three Opus agents on high effort, 7-21 M each, 5-12
  active minutes) and decided what four long builder runs had been stuck on
  (`Checked` tables, `Flow`/`Build`, the check core). Count them against the
  builders they unblocked, not against moved lines.
* **Not measured:** the orchestrating session's own context, test and build
  time (the machine is shared), review effort. Add them when they can be
  counted.

### Target to beat

Context tokens per moved Rust line below 70k on the short-agent slices while
the share keeps rising by 0.2-0.5 points per commit. If a snapshot is worse,
find out why before changing the rules (CLAUDE.md, "Measuring performance":
understand a result before quoting it).

## 2026-10-11, later (0.0.787-0.0.823)

Share 43.8 % -> 54.3 % (`scripts/self_hosting.py`), 35k Rust and 41.5k Nikaia
code lines. The agents table of this snapshot (the per-commit table is
`scripts/tokenspeed.py`'s first table, rerun it; the plan agents are "plan"):

#### Agents

| agent | module | turns | context tokens read (M) | written (k) | active min | Rust lines moved in its window | tokens per moved line |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| a0417f | emit | 164 | 28.3 | 1.4 | 28 | 0 | - |
| a0bc5a | emit | 110 | 10.4 | 5.0 | 24 | -181 | - |
| a0f5f3 | check | 191 | 29.1 | 1.6 | 29 | 578 | 50,405 |
| a1bcd3 | check | 118 | 14.2 | 9.1 | 28 | 162 | 87,598 |
| a5fd67 | check | 22 | 1.4 | 0.3 | 9 | 0 | - |
| a7bf2b | emit | 103 | 11.7 | 1.2 | 23 | 215 | 54,454 |
| a965bc | emit | 156 | 23.1 | 1.4 | 36 | 192 | 120,393 |
| a98554 | check | 139 | 17.1 | 1.2 | 20 | 0 | - |
| a99ad9 | check | 182 | 24.1 | 2.0 | 41 | 659 | 36,608 |
| aa4345 | check | 122 | 14.5 | 1.3 | 34 | 378 | 38,474 |
| aa737a | plan | 72 | 6.9 | 0.6 | 5 | 0 | - |
| aa986c | plan | 102 | 9.8 | 0.8 | 11 | 0 | - |
| aad2bf | check | 96 | 9.4 | 1.0 | 25 | -104 | - |
| ab62ea | emit | 695 | 219.1 | 6.5 | 315 | 1167 | 187,742 |
| ab8a58 | check | 768 | 276.2 | 6.9 | 281 | 2292 | 120,518 |
| aba0cc | plan | 318 | 71.3 | 2.8 | 53 | 0 | - |
| abfc7f | emit | 173 | 26.0 | 1.6 | 41 | 450 | 57,730 |
| ac786f | plan | 124 | 16.0 | 2.1 | 10 | 0 | - |
| ae1cf5 | check | 156 | 20.5 | 1.3 | 33 | 418 | 48,963 |
| af7610 | plan | 159 | 20.9 | 1.5 | 12 | 0 | - |
| af8991 | emit | 154 | 22.3 | 1.3 | 37 | 136 | 163,867 |
| afd84f | emit | 33 | 2.0 | 0.5 | 15 | 0 | - |

### Reading

* **Two cohorts.** Long agents (ab62ea, ab8a58; one for 5 h, one for 4.7 h):
  495 M context tokens for 3,459 lines moved = **143k per moved line**.
  Fresh one-slice agents (11 builders from a99ad9 on): 205 M for 2,903 net
  lines = **71k per moved line**, about half. The scaffolding units (C-3, U0b)
  count negative lines and are inside that figure; the slices that really move
  code cost 37k-54k (a99ad9, aa4345, ae1cf5, a0f5f3, a7bf2b), the entangled
  ones 88k-164k (a1bcd3, a965bc, af8991).
* **Speed.** The fresh builders moved about 500 net lines per active hour
  (2,903 lines in 5.9 h), the two long agents about 350 (3,459 in 9.9 h).
  Two agents ran in parallel, so the wall-clock rate is about double that.
  Share per commit: +0.3 points before 0.0.810, +0.3-0.6 after.
* **Not yet a trend.** What is left is the harder part: the cycles of `check`
  (24 functions) and `emit` (47) and the Rust services below them. The plan
  agents (aba0cc 71 M, 318 turns, the rewrite script, bump script, defect
  list) are the one long run since; `rs2nika.py` saved about one turn per unit
  (C-5b), the twin file in the other module saved more.
* **Waste measured:** two hours of idle agents after an interruption, seven
  red CI runs from a date mismatch (fixed, `scripts/bump.py` now sets it),
  version collisions on most rebases (`bump.py rebase-fix`).

## Runs

One row per agent, from `scripts/tokenspeed.py --short AGENTS_DIR` after it
ends, with the builder's rs2nika line.

| date | agent | module | turns | context M | Rust lines moved | tokens per moved line | rs2nika |
| :--- | :--- | :--- | ---: | ---: | ---: | ---: | :--- |
| 2026-10-24 | a312a5 | fmt (#101 step 4) | 38 | 1.2 | 0 | - | - |
| 2026-10-24 | a66fd3 | check (fold_bound) | 185 | 14.4 | 1346 | 10,732 | - |
| 2026-10-24 | af07ce | check | 99 | 8.5 | 1024 | 8,295 | - |
| 2026-10-24 | adc4be | check | 89 | 6.4 | 569 | 11,185 | - |
| 2026-10-24 | a88404 | check | 75 | 5.3 | 353 | 14,935 | - |

Moved lines are attributed by `scripts/tokenspeed.py`: commits in the agent's
window that changed a file it edited (Edit/Write/`git add` in its transcript)
or whose subject names its id. Files edited by two agents in one window are
counted for both, so a row can be too high; the 0 rows are agents that
committed no `crates/nikaia/src` Rust. The turn count is assistant messages in
the transcript (a66fd3 shows 185, not about 100). The full table also has
"read after 150k / 300k" columns: no agent but a96a76 (75 % / 2 %) read
meaningfully past 150k, so context size is not what made the long agents dear.
