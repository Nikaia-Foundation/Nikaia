# How we build Nikaia

<style>
.hwb-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(15rem,1fr));gap:.9rem;margin:1.2rem 0}
.hwb-tile{display:block;padding:1rem 1.1rem;border:1px solid var(--nk-border-strong,#d8dde5);border-radius:10px;background:var(--nk-surface,#f5f7fa);color:var(--nk-fg,#1f2733);text-decoration:none;border-top:3px solid var(--nk-accent,#a57c24)}
.hwb-tile:hover,.hwb-tile:focus{border-color:var(--nk-link,#1a67a3);text-decoration:none}
.hwb-num{display:block;font-size:1.9rem;font-weight:700;line-height:1.15;color:var(--nk-accent,#a57c24)}
.hwb-lbl{display:block;margin-top:.3rem}
.hwb-more{display:block;margin-top:.5rem;font-size:.85rem;color:var(--nk-link,#1a67a3)}
.hwb-detail{padding:.2rem 1rem .6rem;margin:1rem 0;border-left:3px solid var(--nk-border-strong,#d8dde5);scroll-margin-top:4rem}
.hwb-detail:target{border-left-color:var(--nk-accent,#a57c24);background:var(--nk-surface,#f5f7fa)}
.hwb-detail h3{margin-top:.6rem}
.hwb-scroll{overflow-x:auto}
.hwb-scroll table{min-width:44rem;border-collapse:collapse}
.hwb-scroll th,.hwb-scroll td{vertical-align:top;padding:.4rem .6rem;border-bottom:1px solid var(--nk-border,#e2e6ec);text-align:left}
</style>

This page says how the work is organised, what is measured, and what is not
yet. It describes a way of working that is still changing.

## Introduction

The owner sets the goals, the stages and the priorities, and the standard is
high on purpose: a defect met on the way is fixed in the compiler, with a
test, as its own commit, never worked around. Nothing reaches `main` unless
it passed CI.

The owner talks to one session, the **main session**. It hands the questions
to the **architect** (an Opus agent with few tools: it reads, edits documents,
works on GitHub issues, and returns one work order). The main session starts a
**builder** from that order: six file and shell tools, no GitHub, one or two
steps, then commit, push, report, end. The goal behind the work is that
Nikaia hosts itself; the converter `crates/rs2nika` writes the first draft of
each Rust function in Nikaia, and the builder starts from it.

The cost is **tokens**: an agent pays for the context it re-reads on every
turn, not for what it writes. So we count tokens, and instructions for
performance; never seconds.

## Current state

Click a tile for what the number counts, how it was taken, and what it does not show.

<div class="hwb-grid">
<a class="hwb-tile" href="#share"><span class="hwb-num">69.4 %</span><span class="hwb-lbl">of the toolchain is written in Nikaia</span><span class="hwb-more">details</span></a>
<a class="hwb-tile" href="#perline"><span class="hwb-num">143k &rarr; 71k</span><span class="hwb-lbl">context tokens per moved Rust line, long agents against restarted ones</span><span class="hwb-more">details</span></a>
<a class="hwb-tile" href="#context"><span class="hwb-num">566-626k</span><span class="hwb-lbl">tokens of context at the end of a long agent; 71-78 % of its cost came after 300k</span><span class="hwb-more">details</span></a>
<a class="hwb-tile" href="#builder"><span class="hwb-num">52k &rarr; ~15k</span><span class="hwb-lbl">context at the start of a builder with six tools instead of about forty</span><span class="hwb-more">details</span></a>
<a class="hwb-tile" href="#tests"><span class="hwb-num">267 &rarr; 1</span><span class="hwb-lbl">test binaries (16-20 MB each); now one, <code>--test it</code></span><span class="hwb-more">details</span></a>
<a class="hwb-tile" href="#package"><span class="hwb-num">1 of 256</span><span class="hwb-lbl">objects changed on a one-file change; was one 7.4 MB unit</span><span class="hwb-more">details</span></a>
</div>

<div class="hwb-detail" id="share" markdown="1">

### 69.4 % in Nikaia

`python3 scripts/self_hosting.py` counts code lines (not blank, not only a
comment) in `tools/*.nika` against the Rust of the toolchain. Today (0.0.1210) that is
72,529 lines of Nikaia (71,944 at 0.0.1207, 68.9 %). It was 43.8 % at 0.0.787 and 54.3 % at 0.0.823 on
2026-10-11. It moves with every step, so the figure on this page is the day
it was written. It does not say how much of the *hard* part is left: what
remains are the cycles of `check` and `emit` and the services below them.
Lines are not effort, and a slice that only declares state moves nothing.

</div>

<div class="hwb-detail" id="perline" markdown="1">

### 143k against 71k tokens per moved line

Context tokens read, divided by the Rust code lines an agent's commits removed
(`scripts/tokenspeed.py`, ledger in `docs/tokenspeed.md`, snapshot of
2026-10-11). Two long agents: 495 M tokens for 3,459 lines, 143k per line.
Eleven fresh one-slice builders: 205 M for 2,903 net lines, 71k per line. A
clean slice cost 37-54k, an entangled one 88-164k. The target is below 50k.
It does not show: the main session's own context, build and test cost, review
effort. Two cohorts are not yet a trend, and the work that is left is harder
than the work that was moved.

</div>

<div class="hwb-detail" id="context" markdown="1">

### 566-626k tokens of context

Measured on the two long agents of 2026-10-11 (695 and 768 turns, 219 M and
276 M tokens read): the context started near 49k, grew about 750 tokens a
turn, and ended at 566-626k. Because each turn re-reads everything, 71-78 % of
the cost came after the context passed 300k. If the same work had been done
by agents restarted every 120 turns it would cost about one third. That last
figure is **computed, not confirmed**; the 71k column above is the first
evidence for it.

</div>

<div class="hwb-detail" id="builder" markdown="1">

### 52k to about 15k at the start

Ten build agents measured on 2026-10-17 started with 52k tokens of context, of
which 35-40k were descriptions of 41 tools a builder never calls. The
`builder` type (`.claude/agents/builder.md`) has six tools. The about-15k
figure is the size `CLAUDE.md` gives for a builder with that set; it is
**not yet re-measured** on a run. Growth per turn was about 1k, half of it
thinking. Seven of ten agents ran past 200 turns, and 10-20 % of the cost went
on polling a build or a test: waiting is now one call.

</div>

<div class="hwb-detail" id="tests" markdown="1">

### 267 test binaries become one

Before 0.0.1207 the 267 test files in `crates/nikaia/tests` were 267
binaries, 16-20 MB each at the fast profile, because each linked the compiler
and `std`: about 4.5 GB per profile and 267 links after a change
(`docs/modular-compiler.md`). Commit 539ea132 merged them into one
integration binary, `tests/it` (#793 step B); `--test a` became a filter,
`cargo test --profile fast -p nikaia --test it a::`. The 267 -> 1 is a count
of binaries; the saving in links is read from the tree, the saving in disk
is not re-measured here.

</div>

<div class="hwb-detail" id="package" markdown="1">

### One Rust module per `.nika`

`package.rs` (73k lines, 3,300 functions, one 7.4 MB object) is gone. Since
0.0.1207 `lower-std` writes one generated file per `.nika` in
`crates/nikaia-std/src/tools/lowered/`, with `lowered.rs` as the index
(#667 step 4, #793). The commit measures it: changing one body changed
**1 of 256** incremental objects (78 KB); the largest object is 1.6 MB.
Counted in objects and bytes, not in time.

</div>

## Iterations

In order of time. "Measured" means a number is in the repository; "not
measured yet" means nobody has counted.

<div class="hwb-scroll" markdown="1">

| When | Measure taken | What improved |
| :--- | :--- | :--- |
| 2026-10-11 | Self-hosting by long agents (0.0.787-0.0.810) | Share 43.8 % to 49.8 % in 24 commits; 120-190k tokens per moved line, which is the cost we then found too high |
| 2026-10-11 | Short Opus planning agents decide what builders were stuck on | 7-21 M tokens each, against 219-276 M for a long builder; value counted in the builders unblocked, not measured as lines |
| 2026-10-11 | One agent, one slice, restart at about 120 turns | 71k against 143k per moved line (0.0.787-0.0.823); share 54.3 % |
| 2026-10-11 | `rs2nika` draft as the default start of a move | About one turn saved per unit (C-5b); the twin file in the other module saved more |
| 2026-10-11 | Research brief (80 lines) before the builder | Not measured yet |
| 2026-10-17 | Ten build agents measured | Found 52k at start, 35-40k of it unused tool descriptions; 10-20 % of cost spent polling |
| 2026-10-17 | `builder` agent type with six tools; waiting in one call | Start context about 15k per `CLAUDE.md`; not re-measured on a run yet |
| (#724) | `--profile fast`: release optimisation, incremental | After a one-file change the `nikaia` binary takes about 18 s instead of 70 s, per `CLAUDE.md`; a count of units is not taken yet |
| 2026-10-24 | Rules: count, never time; callgrind for performance | A rule; the same binary varied 1.4-1.9 times between days |
| 2026-10-24 | Architect role, main session as carrier, stage 1: one builder at a time (`docs/architect.md`) | Running. First run rows in `docs/tokenspeed.md`: a312a5, 38 turns, 1.2 M tokens, no line moved. Not enough to judge |
| 2026-10-24 | Decision basis for a modular compiler (#793) | Planned: one module per `.nika` in `package.rs` (#667 step 4), one test binary, then stage 2 with two builders. Nothing built, nothing measured |
| (#667) | Per-file cache keys for the lowering | Built (cbb86798, bc65ff63): the key is the compiler's identity, then its fingerprint with rustc and `Cargo.lock`. A review found a stale-hit risk in the first form and filed it as #801 before it caused harm |
| 2026-10-24 | `lower-std`: one module per `.nika` (0.0.1207, ADR-294 D18/D19) | 1 of 256 objects changed on a one-file change, was one 7.4 MB unit |
| 2026-10-24 | Test binaries 267 -> 1 (`--test it`, 539ea132) | Count of binaries; links per change 267 -> 1 |
| 2026-10-24 | CI: tests built in a step of their own (#795 step 1) | Separates compiling from running. A slip on the way: an unquoted `accepted_sweep:: -- --ignored` in the workflow was a YAML mapping error; fixed in 8d56f29c |
| 2026-10-24 | CI findings, three develop runs (`docs/ci-cache.md`) | Up to 40 % of a run is queue wait (one run 40 %, another 14 %); `Test` is 68-83 % of a job. The owner parked the cache plan until the build is more cacheable |
| 2026-10-24 | Self-hosting, U-d4 (check_function.nika, 3 steps) | Share 68.9 % at 0.0.1207, 69.4 % at 0.0.1210 (72,529 lines). The 520-line reduction of `check/mod.rs` is not re-counted here |
| 2026-10-24 | Named properties, ADR-381 (0.0.1208-0.0.1210) | A pure bool call is a fact the prover reads by name; `binary_search` requires `is_sorted()` and `sort` ensures it (D5); a proof resting on an `assume` is marked in `--bounds` and `--asserts` (D6) |
| 2026-10-24 | Measure of the compiler's own code (`docs/formal-verification.md`) | Indexes 802 of 1,473 proved (54 %); arithmetic 491 of 1,101 (45 %). The rest keep their run-time checks |

</div>

## What we learned on the way

* A binary search needs lengths and counters (`0 <= lo <= hi <= len`), not
  sortedness, to be free of index errors (`docs/facts-about-data.md`);
  sortedness matters for the answer, not for the bounds.
* A property such as "sorted" can be expressible in the language and still
  unreadable to the prover; naming it (ADR-381) made it a fact.
* Reading a decision basis with the owner changed the claim: the owner chose
  the honest wording over "formally verified".
* The owner takes one question at a time in a decision round.

## The claim we make

The compiler's own Nikaia code is proved free of run-time errors **in part**,
and measured: 802 of 1,473 indexes and 491 of 1,101 arithmetic steps today.
It is not complete, and we do not say the compiler is formally verified.

## What is not proven

That the restarted-agent way is cheaper over a whole project (two cohorts, one
snapshot). That six tools bring the start to about 15k. That a modular compiler
shortens the cycle, and that two builders can run without conflicts.

## The language builds itself

The goal is that Nikaia hosts itself. Today 69.4 % of the toolchain is
written in Nikaia (`scripts/self_hosting.py`; the figure moves with every
step). A converter, `crates/rs2nika`, translates a Rust function into a first
draft in Nikaia, marks with TODO where it cannot, and names the functions it
still needs. The builder starts from that draft, not from an empty file.

Every helper tool is written in Nikaia, not in Python or shell. That is a
principle, not a convenience: whatever blocks or chafes on the way, a missing
library entry, a gap in the syntax, a defect in the lowering, is a finding,
and the finding is fixed in the compiler.

Where we stand is on the [changelog](CHANGELOG.html); the reasons for the
project are in the [manifesto](manifesto.html).
