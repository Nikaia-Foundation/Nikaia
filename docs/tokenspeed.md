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
