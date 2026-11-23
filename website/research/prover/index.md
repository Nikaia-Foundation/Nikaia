# Open questions about the prover

The compiler proves claims about a program while it builds it. An index it
proves to stay inside its list, or an arithmetic step it proves not to
overflow, needs no check at run time. An `assert` it proves costs nothing when
the program runs, and by default an `assert` it cannot prove refuses the
program ([ADR-380](/docs/specification/adr/adr-380.html)). The solver behind this is not taken on
trust, because a small checker replays a certificate for every proof before a
check is dropped ([ADR-270](/docs/specification/adr/adr-270.html) D5).

Proving that an index check can be left out is not new in itself. What these
questions ask is how far it carries when it is the compiler's ordinary
behaviour, for ordinary programs, with nothing written for the prover's sake,
and what that demands of the language and of the prover. The internal
structure they lean on is described in
[how the prover is built](architecture.html). Whether a proved check is then
left out of the program, and what it takes to trust a proof that far, is asked
under [performance with safety](../performance/).

## Where the prover stands today

The compiler's own code written in Nikaia (`tools/*.nika`) is the first test
bed. The goal that it be free of run-time errors is
[#796](https://github.com/Nikaia-Foundation/Nikaia/issues/796), and the work towards it is [#1053](https://github.com/Nikaia-Foundation/Nikaia/issues/1053). A count taken
for #1053 file by file found 965 of 1,821 indexes proved (53 %) and 560 of
1,381 arithmetic steps (41 %). The issue notes that the count misses sites in
files the single-file check could not type, so the figure is a lower bound
with a known gap and not yet a final number.

<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="Indexes proved in the compiler's Nikaia code, 965 of 1821, 53 %"><span style="width: 53%"></span></div><p class="rq-note">Indexes proved in the compiler's Nikaia code, 965 of 1,821 (53 %), counted for #1053.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="Arithmetic steps proved, 560 of 1381, 41 %"><span style="width: 41%"></span></div><p class="rq-note">Arithmetic steps proved, 560 of 1,381 (41 %), counted for #1053.</p></div>

The solver package `logic`, written in Nikaia, is the second test bed. A
count with `nikaia check --bounds` on 2026-11-23 found 624 of its 1,353
indexes proved (46 %) and 381 of 777 arithmetic steps. Most of the indexes
left open fall into a few groups. In 219 no fact bounds the value at all,
and in 173 the index is itself read out of another list, such as a variable's
position in a heap that is looked up in a second list. In 86 a loop runs
over the length of one list and indexes another, and 36 index a list of
lists. The indexes read out of another list are the next step
([#1890](https://github.com/Nikaia-Foundation/Nikaia/issues/1890)): a survey of what each needs, then element rules the
compiler finds by itself, then a theory of arrays for the pairs of lists
that point at each other.

<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="Indexes proved in the solver package logic, 624 of 1353, 46 %"><span style="width: 46%"></span></div><p class="rq-note">Indexes proved in the solver package <code>logic</code>, 624 of 1,353 (46 %), counted on 2026-11-23.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="Arithmetic steps proved in the solver package logic, 381 of 777, 49 %"><span style="width: 49%"></span></div><p class="rq-note">Arithmetic steps proved in the solver package <code>logic</code>, 381 of 777 (49 %), counted on 2026-11-23.</p></div>

Each question below carries two bars. The first shows how far the work is.
Its filled part <span class="rq-key rq-key-done"></span> is what is built,
the plain part <span class="rq-key rq-key-known"></span> is the rest of the
scope we know of, including what we know we do not know yet, and the hatched
part <span class="rq-key rq-key-unknown"></span> is a reserve for what we do
not yet know we will meet, since such things turn up in every piece of this
work. The second bar shows the evidence and starts in the middle. To the left
it points to the opposite answer, to the right
(<span class="rq-key rq-key-conf"></span>) to a positive one, and its right
end would be an answer shown by measured figures. Both are the project's
estimates. Under each question a short note says what they rest on, and they
change when the work or the evidence does.

## Against z3 and cvc5

The solver behind the prover is also measured on public problems, against z3
and cvc5. The set is 578 QF_LIA and 519 QF_LRA files of SMT-LIB, taken from
GitHub at fixed commits (`tests/solver/SOURCES.md`), each one query with a
known answer. Each solver gets 10 seconds per file. The goal, per family and
per logic, is to solve every file z3 solves within that limit, with no wrong
answer. Across the set there are two more: no wrong answer on any file, and
at most z3's instructions per solved file on the 50-file sample CI runs
(median and total). These figures were measured on 2026-11-20 with version
0.0.1481, against z3 5.3.3 and cvc5 1.4.2. The full table, with times, memory
and the method, is §9 of
[`docs/solver-workload.md`](https://github.com/Nikaia-Foundation/Nikaia/blob/develop/docs/solver-workload.md#9-against-z3-and-cvc5-on-smt-lib-1338).

A family is reached where ours solves at least as many files as z3. Each bar
below fills ours' share <span class="rq-key rq-key-conf"></span>, with z3's
share marked by a line <span class="rq-key rq-key-goal"></span>. A family
behind its goal is filled in the muted colour <span class="rq-key rq-key-behind"></span>.
Most of what is left are files whose numbers leave the 64-bit integers the
solver computes in. It answers those *unknown* on purpose rather than
guess, and keeps them counted ([ADR-270](/docs/specification/adr/adr-270.html) D11).

**Reached.** Two goals hold across the set:

<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="Files answered without a wrong answer: 1097 of 1097 (100 %), the goal is all of them"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">No wrong answer on the 1,097 files, and no disagreement with z3 or cvc5.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="Median instructions per solved file of the CI sample: ours 3.2 M instructions, 23 % of z3's 14.0 M; the goal is at most z3's"><span style="width: 22.9%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">Median instructions per file over the 47 sample files all three solve: ours 3.2 M, z3 14.0 M, cvc5 18.9 M. The bar is ours as a share of z3's; the mark is z3's own figure, so a bar that ends before it is ahead.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="Total instructions over the CI sample: ours 509 M instructions, 44 % of z3's 1167 M; the goal is at most z3's"><span style="width: 43.6%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">Total over the same 47 files: ours 509 M, z3 1,167 M, cvc5 2,367 M. Ours costs fewer instructions than z3 on each of the sample's four families.</p></div>

These families are level with z3 or ahead of it (the bar is ours' share of the family's files, the mark z3's):

<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LRA seeds: ours solves 499 of 499 files (100 %), the goal is z3's 499 (100 %)"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LRA seeds: ours 499 of 499, as z3 and cvc5.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LIA 20180326-Bromberger: ours solves 1 of 1 files (100 %), the goal is z3's 1 (100 %)"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LIA 20180326-Bromberger: ours 1 of 1, as z3 and cvc5.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LIA CAV_2009_benchmarks: ours solves 1 of 1 files (100 %), the goal is z3's 1 (100 %)"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LIA CAV_2009_benchmarks: ours 1 of 1, as z3 and cvc5.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LIA bofill-scheduling: ours solves 2 of 2 files (100 %), the goal is z3's 2 (100 %)"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LIA bofill-scheduling: ours 2 of 2, as z3 and cvc5.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LIA rings: ours solves 1 of 1 files (100 %), the goal is z3's 1 (100 %)"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LIA rings: ours 1 of 1, as z3 and cvc5.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LIA rings_preprocessed: ours solves 2 of 2 files (100 %), the goal is z3's 2 (100 %)"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LIA rings_preprocessed: ours 2 of 2, as z3 and cvc5; one file more than on 2026-11-18.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LIA nec-smt: ours solves 1 of 3 files (33 %), the goal is z3's 1 (33 %)"><span style="width: 33.3%"></span><span class="rq-metric-goal" style="left: 33.3%"></span></div><p class="rq-note">QF_LIA nec-smt: ours 1 of 3, as z3; cvc5 0. Neither solver answers the other two within the limit.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LIA 20231117-c_inference: ours solves 30 of 31 files (97 %), the goal is z3's 30 (97 %)"><span style="width: 96.8%"></span><span class="rq-metric-goal" style="left: 96.8%"></span></div><p class="rq-note">QF_LIA 20231117-c_inference: ours 30 of 31, as z3; cvc5 12. No solver answers the last file within the limit. The three files ours took past it are answered since the simplex ranks the entering variable by its column's size times its rows (<a href="https://github.com/Nikaia-Foundation/Nikaia/issues/2401">#2401</a>).</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LRA clock_synchro: ours solves 3 of 4 files (75 %), the goal is z3's 3 (75 %)"><span style="width: 75%"></span><span class="rq-metric-goal" style="left: 75%"></span></div><p class="rq-note">QF_LRA clock_synchro: ours 3 of 4, as z3 and cvc5 (measured again on 2026-11-20, <a href="https://github.com/Nikaia-Foundation/Nikaia/issues/2400">#2400</a>): two files that left <code>i64</code> are proved since the simplex combines a pivot's rows in double width (<a href="https://github.com/Nikaia-Foundation/Nikaia/issues/2307">#2307</a>). The fourth, which no solver answers within the limit, still leaves <code>i64</code>: its rows do not fit even reduced by their gcd.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LRA keymaera: ours solves 2 of 2 files (100 %), the goal is z3's 2 (100 %)"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LRA keymaera: ours 2 of 2, as z3 and cvc5.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LRA sal: ours solves 3 of 3 files (100 %), the goal is z3's 3 (100 %)"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LRA sal: ours 3 of 3, as z3 and cvc5.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LRA spider_benchmarks: ours solves 2 of 2 files (100 %), the goal is z3's 2 (100 %)"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LRA spider_benchmarks: ours 2 of 2, as z3 and cvc5.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LRA tta_startup: ours solves 2 of 2 files (100 %), the goal is z3's 2 (100 %)"><span style="width: 100%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LRA tta_startup: ours 2 of 2, as z3 and cvc5; both are new since 2026-11-18.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LRA uart: ours solves 1 of 2 files (50 %), the goal is z3's 1 (50 %)"><span style="width: 50%"></span><span class="rq-metric-goal" style="left: 50%"></span></div><p class="rq-note">QF_LRA uart: ours 1 of 2, as z3 and cvc5; the other file none of the three answers within the limit.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="QF_LRA 2017-Heizmann: ours solves 0 of 4 files (0 %), the goal is z3's 0 (0 %)"><span style="width: 0%"></span><span class="rq-metric-goal" style="left: 0%"></span></div><p class="rq-note">QF_LRA 2017-Heizmann: none of the three answers these 4 files; ours answers each <code>unknown (overflow)</code>, since they leave <code>i64</code>.</p></div>

**Behind.** These families are below z3's count; the note names the reason and the issue that works on it:

<div class="rq-metric"><div class="rq-metric-bar rq-behind" role="img" aria-label="QF_LIA 20220307-SMPT: ours solves 362 of 371 files (98 %), the goal is z3's 367 (99 %)"><span style="width: 97.6%"></span><span class="rq-metric-goal" style="left: 98.9%"></span></div><p class="rq-note">QF_LIA 20220307-SMPT: ours 362 of 371, z3 367, cvc5 368. Nine files run to the limit, three of which no solver answers within it; the SMPT models that still hit the 10 s limit are <a href="https://github.com/Nikaia-Foundation/Nikaia/issues/2086">#2086</a>. Ours solves 23 files more than on 2026-11-18.</p></div>
<div class="rq-metric"><div class="rq-metric-bar rq-behind" role="img" aria-label="QF_LIA seeds: ours solves 115 of 164 files (70 %), the goal is z3's 162 (99 %)"><span style="width: 70.1%"></span><span class="rq-metric-goal" style="left: 98.8%"></span></div><p class="rq-note">QF_LIA seeds: ours 115 of 164, z3 162, cvc5 157. 48 files leave <code>i64</code> and are answered <code>unknown (overflow)</code> on purpose (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D11); the integer elimination's double-width row combination, <a href="https://github.com/Nikaia-Foundation/Nikaia/issues/2307">#2307</a>, is the work that answers them. One more file runs to the limit.</p></div>
<div class="rq-metric"><div class="rq-metric-bar rq-behind" role="img" aria-label="QF_LIA cut_lemmas: ours solves 0 of 1 files (0 %), the goal is z3's 1 (100 %)"><span style="width: 0%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LIA cut_lemmas: ours 0 of 1, z3 1, cvc5 0. The file leaves <code>i64</code>, answered <code>unknown (overflow)</code> on purpose; the QF_LIA overflows are the work of <a href="https://github.com/Nikaia-Foundation/Nikaia/issues/2307">#2307</a>.</p></div>
<div class="rq-metric"><div class="rq-metric-bar rq-behind" role="img" aria-label="QF_LIA tightrhombus: ours solves 0 of 1 files (0 %), the goal is z3's 1 (100 %)"><span style="width: 0%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LIA tightrhombus: ours 0 of 1, z3 and cvc5 1. The file leaves <code>i64</code>, answered <code>unknown (overflow)</code> on purpose; the QF_LIA overflows are the work of <a href="https://github.com/Nikaia-Foundation/Nikaia/issues/2307">#2307</a>.</p></div>
<div class="rq-metric"><div class="rq-metric-bar rq-behind" role="img" aria-label="QF_LRA LassoRanker: ours solves 0 of 1 files (0 %), the goal is z3's 1 (100 %)"><span style="width: 0%"></span><span class="rq-metric-goal" style="left: 100%"></span></div><p class="rq-note">QF_LRA LassoRanker: ours 0 of 1, z3 and cvc5 1. The file leaves <code>i64</code> in the simplex, answered <code>unknown (overflow)</code> on purpose; why its pivots fail is <a href="https://github.com/Nikaia-Foundation/Nikaia/issues/2101">#2101</a>.</p></div>

Over each logic:

<div class="rq-metric"><div class="rq-metric-bar rq-behind" role="img" aria-label="QF_LIA: ours solves 524 of 578 files (91 %), the goal is z3's 569 (98 %)"><span style="width: 90.7%"></span><span class="rq-metric-goal" style="left: 98.4%"></span></div><p class="rq-note">QF_LIA: ours 524 of 578, z3 569, cvc5 545. Of the 54 files ours leaves open, 38 leave <code>i64</code>, 12 run to the limit and 4 are answered <code>unknown</code>.</p></div>
<div class="rq-metric"><div class="rq-metric-bar rq-behind" role="img" aria-label="QF_LRA: ours solves 512 of 519 files (99 %), the goal is z3's 513 (99 %)"><span style="width: 98.7%"></span><span class="rq-metric-goal" style="left: 98.8%"></span></div><p class="rq-note">QF_LRA: ours 512 of 519, z3 and cvc5 513. Of the 7 files ours leaves open, 6 leave <code>i64</code> and 1 runs to the limit.</p></div>

## What is being built

The solver that is to follow the reference solver is decided part by part in
[ADR-270](/docs/specification/adr/adr-270.html), and each step is measured before the next one
starts (D8). Each part below carries the first bar, which shows how far it is
built, and says what it would let the prover do. A part shown as not built
cannot be used yet. The new engine's parts are built in Nikaia and measured on
SMT-LIB sets; the compiler asks them once [#1285](https://github.com/Nikaia-Foundation/Nikaia/issues/1285) switches it over,
until then it asks the reference solver. How the parts fit together is drawn under
[how the prover is built](architecture.html#the-solver-today-and-as-decided).

<div class="rq-machine">
<p class="rq-layer">Foundation</p>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Certificates and their checker</span><span class="rq-stage rq-stage-built">built</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Certificates and their checker, about 90 % built, 10 % held back for what is not yet known"><span class="rq-known" style="width: 90%"><span class="rq-done" style="width: 100%"></span></span><span class="rq-unknown" style="width: 10%"></span></div><span class="rq-pct">90 %</span></div>
<p class="rq-note">Every proof is replayed step by step before a check is dropped, so no solver has to be trusted. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D1-D5)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name"><code>nikaia.proofs</code> and <code>--locked</code></span><span class="rq-stage rq-stage-built">built</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="<code>nikaia.proofs</code> and <code>--locked</code>, about 90 % built, 10 % held back for what is not yet known"><span class="rq-known" style="width: 90%"><span class="rq-done" style="width: 100%"></span></span><span class="rq-unknown" style="width: 10%"></span></div><span class="rq-pct">90 %</span></div>
<p class="rq-note">Proofs are committed with the code, checked when read, and a locked build searches nothing. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D19-D22)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Reference solver in Nikaia</span><span class="rq-stage rq-stage-built">built</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Reference solver in Nikaia, about 85 % built, 15 % held back for what is not yet known"><span class="rq-known" style="width: 85%"><span class="rq-done" style="width: 100%"></span></span><span class="rq-unknown" style="width: 15%"></span></div><span class="rq-pct">85 %</span></div>
<p class="rq-note">Linear integer arithmetic by Fourier-Motzkin elimination with case splits, the solver every later one is measured against. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D8 step 2)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Loops as Horn clauses</span><span class="rq-stage rq-stage-growing">partly built</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Loops as Horn clauses, about 40 % built, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 53%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><span class="rq-pct">40 %</span></div>
<p class="rq-note">Invariants by closed forms and intervals, each checked, for whole numbers so far. More strategies and outside proposals are decided. (<a href="/docs/specification/adr/adr-361.html">ADR-361</a>)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Proposers, a language model among them</span><span class="rq-stage rq-stage-next">next</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Proposers, a language model among them, about 0 % built, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 0%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><span class="rq-pct">0 %</span></div>
<p class="rq-note">Any outside program, a language model or a tool that learns from test runs, may propose a loop invariant through <code>nikaia prove --propose</code>. The compiler checks every proposal before it is used, and a normal build never calls one. (<a href="/docs/specification/adr/adr-361.html">ADR-361</a> D6)</p>
</div>
<p class="rq-layer">The engine</p>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Solver foundations</span><span class="rq-stage rq-stage-growing">partly built</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Solver foundations, about 55 % built, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 73%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><span class="rq-pct">55 %</span></div>
<p class="rq-note"><code>u32</code> variables, sparse rows, big numbers where a value outgrows 32 bits, and the one interface every theory sits behind. Built except the big numbers: a step that would leave <code>i64</code> answers <em>unknown</em> for now. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D8 step 3, D10-D12)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Fast paths for simple claims</span><span class="rq-stage rq-stage-growing">built, not yet in the compiler</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Fast paths for simple claims, about 60 % built, 20 % held back for what is not yet known"><span class="rq-known" style="width: 80%"><span class="rq-done" style="width: 75%"></span></span><span class="rq-unknown" style="width: 20%"></span></div><span class="rq-pct">60 %</span></div>
<p class="rq-note">The compiler's own questions are tiny, mostly bounds on one variable or the difference of two. These answer them without the general search and without allocating. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D9, D12)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">CDCL(T) skeleton</span><span class="rq-stage rq-stage-growing">built, not yet in the compiler</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="CDCL(T) skeleton, about 60 % built, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 80%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><span class="rq-pct">60 %</span></div>
<p class="rq-note">Decisions, unit propagation, conflict analysis, backjumping and learned clauses, so a question with many cases learns from each dead end instead of trying them one after another. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D12, D16)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Simplex over the integers</span><span class="rq-stage rq-stage-growing">built, not yet in the compiler</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Simplex over the integers, about 60 % built, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 80%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><span class="rq-pct">60 %</span></div>
<p class="rq-note">Larger systems of inequalities than elimination handles well, with cuts for whole numbers and certificates the checker can replay. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D5, D11)</p>
</div>
<p class="rq-layer">Where theories meet</p>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">E-graph and model-based combination</span><span class="rq-stage rq-stage-later">later</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="E-graph and model-based combination, about 0 % built, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 0%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><span class="rq-pct">0 %</span></div>
<p class="rq-note">Equality with uninterpreted functions. Equal inputs give equal results, and theories agree on shared terms through it. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D13)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Arrays</span><span class="rq-stage rq-stage-later">later</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Arrays, about 0 % built, 35 % held back for what is not yet known"><span class="rq-known" style="width: 65%"><span class="rq-done" style="width: 0%"></span></span><span class="rq-unknown" style="width: 35%"></span></div><span class="rq-pct">0 %</span></div>
<p class="rq-note">Read-over-write lemmas on demand, the way towards reasoning about what a list holds, such as a sort that keeps its elements and leaves them in order. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D13)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Bit-vectors</span><span class="rq-stage rq-stage-later">later</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Bit-vectors, about 0 % built, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 0%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><span class="rq-pct">0 %</span></div>
<p class="rq-note">Masks, shifts and wrapping arithmetic, blasted into clauses for the CDCL skeleton. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D13)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Rules over every element</span><span class="rq-stage rq-stage-growing">partly built</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Rules over every element, about 10 % built, 35 % held back for what is not yet known"><span class="rq-known" style="width: 65%"><span class="rq-done" style="width: 15%"></span></span><span class="rq-unknown" style="width: 35%"></span></div><span class="rq-pct">10 %</span></div>
<p class="rq-note">Claims that hold for every element of a list, used by the solver without making proofs fragile. A bound on every element of a field's or parameter's list is found by the compiler and built; nothing of the kind is written by hand. Element rules for values read out of another list and a theory of arrays are next. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a>, <a href="/docs/specification/adr/adr-306.html">ADR-306</a> D21, <a href="https://github.com/Nikaia-Foundation/Nikaia/issues/1890">#1890</a>)</p>
</div>
<p class="rq-layer">Steering</p>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Strategy layer and budgets</span><span class="rq-stage rq-stage-growing">partly built</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Strategy layer and budgets, about 40 % built, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 57%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><span class="rq-pct">40 %</span></div>
<p class="rq-note">Reads a question's features before the search and signals during it, picks a method, and stops at a counted budget so how long a build searches does not depend on how fast the machine is. The features, the pipelines they pick and a local search for models are built; their parameters are being tuned. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D14, D25)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Parallel search</span><span class="rq-stage rq-stage-later">later</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Parallel search, about 0 % built, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 0%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><span class="rq-pct">0 %</span></div>
<p class="rq-note">One hard question spread across several cores. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D8 step 7, D18)</p>
</div>
</div>

## The questions

### Does Nikaia find every invariant by itself, with none written by hand?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 35 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 47%"></span></span><span class="rq-unknown" style="width: 25%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 15%"></span></div></div>
<p class="rq-note">So far the prover reads guards, lengths, loops, what callees promise and the rules of types, and named properties are not yet built. About half of the compiler's own indexes are proved and the share has been rising, while the ceiling is not yet measured.</p>
</div>

The prover reads what the code already says, such as comparisons and guards,
the lengths of lists, loop bounds, what a called function promises, and the
rule a type states about its fields ([ADR-269](/docs/specification/adr/adr-269.html) D9,
[ADR-314](/docs/specification/adr/adr-314.html), [ADR-369](/docs/specification/adr/adr-369.html)). It reads loops
through invariants it finds itself, so for the loops it covers nobody has to
write one ([ADR-361](/docs/specification/adr/adr-361.html) D3).

The question is where the ceiling of this lies on real code, and how small
the set of additional means can be that closes the gap. One such means is
decided. A named property such as `is_sorted(xs)` is proved by name and
carried to callers ([ADR-381](/docs/specification/adr/adr-381.html), to be built in
[#797](https://github.com/Nikaia-Foundation/Nikaia/issues/797)). The measure is the share of indexes and arithmetic steps
proved, on the compiler's own code first and then on the examples and
packages, together with the number of lines written only to help a proof.

### Can the compiler one day refuse a program whose index or overflow checks are not all proved?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">So far only the counts that would decide it exist. Today about half of the indexes would be refused, which says the switch cannot be flipped yet and nothing about later.</p>
</div>

Today an `assert` the compiler cannot prove refuses the program, while an
index or an arithmetic step it cannot prove still keeps its check and stops
the program if that check fails at run time. An index shown to fail for a
value the program reaches already refuses it (`refuted-claims`, Part III
13.3). The open question is whether the rule for an unproved `assert` can one
day hold for an unproved index or overflow too, so that a program builds only
when every such step is proved and a setting is needed to build anything
less.

Nothing is decided here. What would decide it is measured. It is how much of
real code is proved without help, what a programmer has to add where it is
not, and whether the language stays practical when every such addition is
required. A first step that changes nothing for anyone else would be to hold
the compiler's own code to that rule once #1053 reaches every site there, so
that it cannot fall back.

### How much of the prover's reach comes from the design of Nikaia?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 30 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 40%"></span></span><span class="rq-unknown" style="width: 25%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
<p class="rq-note">The rules that make arguments and views visible (ADR-views-v4) are being built now. The reach of a small solver is suggestive, but the share owed to the language is not yet measured.</p>
</div>

A prover can only use what the code makes visible. In Nikaia a function
declares whether it keeps, changes or only looks at each argument, and the
compiler refuses a change to a value while a view of it is still in use
([ADR-views-v4](/docs/specification/adr/adr-views-v4.html)). Lengths, guards and types are facts the
prover reads directly. The working assumption is that a language designed for
this needs a smaller prover, and fewer annotations, for the same reach. The
question is how much of the reach measured above comes from the language and
how much from the solver.

### Can a programmer who does not know the prover predict which proofs hold?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 45 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 60%"></span></span><span class="rq-unknown" style="width: 25%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
<p class="rq-note">Refutations with values, the message for an unproved claim and the committed proof file are built. How often a proof is lost by an unrelated edit is not yet measured.</p>
</div>

A proof that holds today and fails after an unrelated edit turns a
refactoring into a puzzle. Several decisions aim at this. A refutation names
the values that break the claim ([ADR-269](/docs/specification/adr/adr-269.html) D8). The message
for an unproved claim says that not proved is not false
([ADR-380](/docs/specification/adr/adr-380.html)). The solver's search may differ between
machines, but what one machine proved is recorded in the committed
`nikaia.proofs` and is not rejected by another ([ADR-270](/docs/specification/adr/adr-270.html)
D4, D19-D22). The question is how often a proof is lost through an edit that
does not touch what it is about, and whether the messages are enough to find
the way back without knowing how the prover works.

### Can the prover reason about what a list holds?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 35 % held back for what is not yet known"><span class="rq-known" style="width: 65%"><span class="rq-done" style="width: 8%"></span></span><span class="rq-unknown" style="width: 35%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">Neither the e-graph nor the theory of arrays is built yet. Of the rules over every element, a found bound on the elements of a field's or parameter's list is built. There is no evidence either way yet.</p>
</div>

The solver today decides linear integer arithmetic. Many arguments about
programs need more. From `i == c` it should follow that `xs[i] == xs[c]`, and
after `xs[i] = v` a read at `k` should give `v` when `k == i` and the old
value otherwise. Equality with uninterpreted functions, built as an e-graph,
and a theory of arrays carry exactly this. The procedure is decided
([ADR-270](/docs/specification/adr/adr-270.html) D13) and not yet built. Beyond it lie claims
about every element, such as a sorted list or a partition, which need a rule
over all elements and invariants over list contents that the compiler has to
find by itself. The form is decided: such a rule is found, not written
([ADR-306](/docs/specification/adr/adr-306.html) D21). The first part is built, a bound on
every element of a field's or parameter's list up to the length of another
list, and the next parts are planned in [#1890](https://github.com/Nikaia-Foundation/Nikaia/issues/1890). Finding such
invariants without help is the hardest part of the question whether Nikaia finds every
invariant by itself, and the place where its limits are most likely to show.

### Can the prover use rules over every element of a list and keep its proofs stable?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 8 % of the estimated whole, 35 % held back for what is not yet known"><span class="rq-known" style="width: 65%"><span class="rq-done" style="width: 12%"></span></span><span class="rq-unknown" style="width: 35%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">The committed proof file, which keeps a found proof from being searched again, is built, and so is a found bound on every element of a field's or parameter's list. A solver with general rules over all values is not, so their stability has not been measured yet.</p>
</div>

Many claims about a program speak of every element at once: every index
in a list of positions is inside the list, every node of a tree is valid,
the result of a sort holds the same elements as its input. A solver reasons
about such claims with rules that hold for all values of a variable. The
established way to use these rules is to pick, by patterns, which values to
put in. It reaches far, and it is also the main reason proofs in this style
are fragile. A different solver version, another random seed or an edit
elsewhere in the program can make a proof that held fail, and keeping it
alive takes work the program does not otherwise need.

Two parts of Nikaia bear on this. A proof that was found is recorded in the
committed `nikaia.proofs` and checked when it is read, so it is not searched
again on the next build ([ADR-270](/docs/specification/adr/adr-270.html) D4, D19-D22). That
removes one cause of fragility, the search that differs between versions and
machines, but it does not help when an edit changes the question. And a
property such as `is_sorted(xs)` travels by name from a promise to its
callers without its body being read ([ADR-381](/docs/specification/adr/adr-381.html)), so many
claims of this kind are carried without any rule over all elements.

The solver itself decides linear integer arithmetic and has no rules over
all values yet. Whether it needs them is to be asked when a claim does
([ADR-270](/docs/specification/adr/adr-270.html)). The first such claim is decided in a
narrow form. A bound on every element of a field's or parameter's list is
found by the compiler from every write to that list, and no written
`xs.all(|x| ...)` stands beside it, because a written rule would hide where
the prover falls short ([ADR-306](/docs/specification/adr/adr-306.html) D21). That form is
built, and a value read out of such a list keeps its bound when the list
later shrinks. Element rules for values read out of another list, and a
theory of arrays for two lists that point at each other, follow in
[#1890](https://github.com/Nikaia-Foundation/Nikaia/issues/1890). Rules of this kind written into a loop by the
programmer were not taken up ([ADR-361](/docs/specification/adr/adr-361.html)). The question is whether rules over every
element can be used in a form whose proofs survive an edit and a new solver
version, with a certificate the small checker can replay, and how much of
the range of the established method that form gives up.

### What does checking every proof cost in an ordinary build?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 30 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 40%"></span></span><span class="rq-unknown" style="width: 25%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 20%"></span></div></div>
<p class="rq-note">Certificates and their checker are built for linear arithmetic, where a question took 4.2 µs to solve and 1.9 µs to check when measured. The theories still to come are not yet measured.</p>
</div>

Every proof is checked by a checker of its own before it is used, and the
checker for linear arithmetic and clauses is to stay under 1,500 lines
([ADR-270](/docs/specification/adr/adr-270.html) D5). The compiler's own questions are small, with
no more than four variables each, and took about 4.2 µs to solve and 1.9 µs
to check a certificate on average when they were measured
([ADR-270](/docs/specification/adr/adr-270.html) D9, [solver workload](/docs/solver-workload.html)).
The question is whether this stays true as the prover grows to the theories
above, so that checking everything costs a build no more than a fraction of
its time.

### How much can large language models help the prover?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">The checked loop problem a proposal would be checked against is built, and taking proposals from outside is not yet. There is no evidence yet.</p>
</div>

A loop invariant may be proposed by any program, a language model among
them, and a proposal is used only after the compiler has checked it
([ADR-361](/docs/specification/adr/adr-361.html) D6). Much of Nikaia's own code is written with
the help of such models.

The interface is decided and not yet built; how it works is described under
[how the prover is built](architecture.html#invariants-from-a-language-model-or-another-program).

The question is how much a proposer adds to what
the compiler finds alone, measured by what is proved with it and without it,
and whether this keeps the cost of a proof off the person who writes the
program.

Others have measured parts of this, and their results set the expectations:

* Pei and others, [Can Large Language Models Reason about Program Invariants?](https://proceedings.mlr.press/v202/pei23a.html)
  (ICML 2023), trained models on code to predict invariants from the source
  alone, without running it, and found predictions comparable to those of a
  dynamic analysis that watched five runs.
* Chakraborty and others, [Ranking LLM-Generated Loop Invariants for Program Verification](https://aclanthology.org/2023.findings-emnlp.614)
  (Findings of EMNLP 2023), found that models often need many attempts before
  a correct invariant comes out, and that a learned ranking of the candidates
  markedly cuts the calls to the verifier.
* Wu, Barrett and Narodytska, [Lemur: Integrating Large Language Models in Automated Program Verification](https://arxiv.org/abs/2310.04870)
  (2024), let a model propose invariants as sub-goals that an automated
  verifier then checks, and prove that this keeps the verification sound, the
  same division of labour as here.
* Wei and others, [InvBench: Can LLMs Accelerate Program Verification with Invariant Synthesis?](https://arxiv.org/abs/2509.21629)
  (2025), check every proposed invariant with a verifier and measure the time
  it saves. Models help on some programs, do not yet beat the classical
  verifier UAutomizer overall, and solve little of the hard part.
* Xin and others, [DeepSeek-Prover-V1.5](https://arxiv.org/abs/2408.08152)
  (2024), train a model on the feedback of a proof checker (Lean 4) and
  search over its proof attempts, the same loop of proposing and checking
  applied to whole proofs.
* Guessing from runs is older than language models:
  [Daikon](https://plse.cs.washington.edu/daikon/) reports likely invariants
  seen in test runs, and [ICE learning](https://doi.org/10.1007/978-3-319-08867-9_5)
  (Garg and others, CAV 2014) learns an invariant from examples,
  counterexamples and implications a checker hands back. Both are the kind of
  proposer the interface takes besides a model.

### Where should the prover spend its effort?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 15 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 21%"></span></span><span class="rq-unknown" style="width: 30%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">The counted budget and the first two methods of the loop portfolio are built, and the strategy layer is not yet. There is no evidence yet.</p>
</div>

A hard question can lead a method into a dead end where another would
succeed quickly. The solver is to have a strategy layer of its own. It reads
a question's features before the search, watches signals during it, and may
switch to another engine or other parameters at any moment, carrying over
everything already derived ([ADR-270](/docs/specification/adr/adr-270.html) D14). Loop invariants
are found by a portfolio that grows, cheapest method first, from closed forms
and intervals to k-induction, guess and check, and property-directed
reachability ([ADR-361](/docs/specification/adr/adr-361.html) D5). Effort is counted in units of
work and not in time, so the same question gets the same answer on every
machine, and the `proving` setting says how much a build may spend
([ADR-306](/docs/specification/adr/adr-306.html), [ADR-270](/docs/specification/adr/adr-270.html) D25).

Two parts are open. One is how a budget is shared between strategies, and
whether engines of another kind, algebraic ones such as linear algebra over a
loop's equations, earn a place in the portfolio. The other is where a larger
budget pays. A proof about a library function is found once, recorded with
the library, and used by every program that calls it, so a library can
afford to spend far more on its own proofs than a program on each build.

### Can the standard library hand out proved facts?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
<p class="rq-note">None of the promises of <code>sort</code> is built yet. The mechanism a promise travels by, named properties, is decided, and whether it lowers the effort above it is not yet measured.</p>
</div>

A program that calls `sort` should be able to build on what sorting
promises, namely that the result is sorted, that it has the same length, and
that it holds the same elements as before. If the standard library states
such properties as facts and proves them once, each program that uses it
starts from proved pieces instead of proving them again. Part of this is
decided. A named property such as `is_sorted(xs)` is carried from a promise
to every caller ([ADR-381](/docs/specification/adr/adr-381.html)), and `sort()` is to promise it,
with `binary_search` requiring it ([#798](https://github.com/Nikaia-Foundation/Nikaia/issues/798)). That the length stays the
same is plain arithmetic for the prover. That the result is a permutation of
the input needs a way to speak about what a list holds as a whole, which is
not yet decided, and proving the sorting functions themselves against all
three needs the facts about list contents described above. The question is
how far a library of proved building blocks lowers the effort in the
programs above it.

## Ideas the prover rests on

| Idea | Recorded in |
| :--- | :--- |
| An `assert` is a contract, proved where possible and a refused program where not | [ADR-269](/docs/specification/adr/adr-269.html), [ADR-380](/docs/specification/adr/adr-380.html) |
| A check is dropped only where a proof shows it cannot fail | [ADR-306](/docs/specification/adr/adr-306.html) |
| What a grammar matched and what a callee promises are facts | [ADR-314](/docs/specification/adr/adr-314.html), [ADR-359](/docs/specification/adr/adr-359.html) |
| A type's rule about its fields is proved where values are built and known where they are read | [ADR-369](/docs/specification/adr/adr-369.html) |
| A pure `bool` call is a property the prover knows by name | [ADR-381](/docs/specification/adr/adr-381.html) |
| A loop is a problem in Horn clauses, and invariants are found, checked, and may be proposed from outside | [ADR-361](/docs/specification/adr/adr-361.html) |
| A function says what it keeps and changes, and a view of a value forbids changing it | [ADR-views-v4](/docs/specification/adr/adr-views-v4.html) |
| The solver is written in Nikaia, untrusted, its proofs checked and committed | [ADR-270](/docs/specification/adr/adr-270.html) |
| A strategy layer switches engines during a search, and effort is counted in work, not time | [ADR-270](/docs/specification/adr/adr-270.html) D14, D25, [ADR-306](/docs/specification/adr/adr-306.html) |
| A library promises properties its callers use as facts | [ADR-381](/docs/specification/adr/adr-381.html), [#798](https://github.com/Nikaia-Foundation/Nikaia/issues/798) |
