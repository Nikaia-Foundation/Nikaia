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

## Where it stands

The compiler's own code written in Nikaia (`tools/*.nika`) is the first test
bed. The goal that it be free of run-time errors is
[#796](https://github.com/Nikaia-Foundation/Nikaia/issues/796), and the work towards it is [#1053](https://github.com/Nikaia-Foundation/Nikaia/issues/1053). A count taken
for #1053 file by file found 965 of 1,821 indexes proved (53 %) and 560 of
1,381 arithmetic steps (41 %). The issue notes that the count misses sites in
files the single-file check could not type, so the figure is a lower bound
with a known gap and not yet a final number.

<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="Indexes proved in the compiler's Nikaia code, 965 of 1821, 53 %"><span style="width: 53%"></span></div><p class="rq-note">Indexes proved in the compiler's Nikaia code, 965 of 1,821 (53 %), counted for #1053.</p></div>
<div class="rq-metric"><div class="rq-metric-bar" role="img" aria-label="Arithmetic steps proved, 560 of 1381, 41 %"><span style="width: 41%"></span></div><p class="rq-note">Arithmetic steps proved, 560 of 1,381 (41 %), counted for #1053.</p></div>

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

## What is being built

The solver that is to follow the reference solver is decided part by part in
[ADR-270](/docs/specification/adr/adr-270.html), and each step is measured before the next one
starts (D8). Each part below carries the first bar, which shows how far it is
built, and says what it would let the prover do. A part shown as not built
cannot be used yet. How the parts fit together is drawn under
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
<div class="rq-comp-head"><span class="rq-comp-name">Solver foundations</span><span class="rq-stage rq-stage-next">next</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Solver foundations, about 5 % built, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><span class="rq-pct">5 %</span></div>
<p class="rq-note"><code>u32</code> variables, sparse rows, big numbers where a value outgrows 32 bits, and the one interface every theory sits behind. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D8 step 3, D10-D12)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Fast paths for simple claims</span><span class="rq-stage rq-stage-then">after that</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Fast paths for simple claims, about 0 % built, 20 % held back for what is not yet known"><span class="rq-known" style="width: 80%"><span class="rq-done" style="width: 0%"></span></span><span class="rq-unknown" style="width: 20%"></span></div><span class="rq-pct">0 %</span></div>
<p class="rq-note">The compiler's own questions are tiny, mostly bounds on one variable or the difference of two. These answer them without the general search and without allocating. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D9, D12)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">CDCL(T) skeleton</span><span class="rq-stage rq-stage-then">after that</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="CDCL(T) skeleton, about 0 % built, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 0%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><span class="rq-pct">0 %</span></div>
<p class="rq-note">Decisions, unit propagation, conflict analysis, backjumping and learned clauses, so a question with many cases learns from each dead end instead of trying them one after another. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D12, D16)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Simplex over the integers</span><span class="rq-stage rq-stage-then">after that</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Simplex over the integers, about 0 % built, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 0%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><span class="rq-pct">0 %</span></div>
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
<p class="rq-layer">Steering</p>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Strategy layer and budgets</span><span class="rq-stage rq-stage-later">later</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Strategy layer and budgets, about 5 % built, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><span class="rq-pct">5 %</span></div>
<p class="rq-note">Reads a question's features before the search and signals during it, picks a method, and stops at a counted budget so how long a build searches does not depend on how fast the machine is. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D14, D25)</p>
</div>
<div class="rq-comp">
<div class="rq-comp-head"><span class="rq-comp-name">Parallel search</span><span class="rq-stage rq-stage-later">later</span></div>
<div class="rq-comp-bar"><div class="rq-build" role="img" aria-label="Parallel search, about 0 % built, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 0%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><span class="rq-pct">0 %</span></div>
<p class="rq-note">One hard question spread across several cores. (<a href="/docs/specification/adr/adr-270.html">ADR-270</a> D8 step 7, D18)</p>
</div>
</div>

## The questions

### How far does it reach with nothing written for it?

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

### Can an unproved check one day refuse the program?

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

### How much does the design of the language carry?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 30 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 40%"></span></span><span class="rq-unknown" style="width: 25%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
<p class="rq-note">The rules that make arguments and views visible (ADR-views-3) are being built now. The reach of a small solver is suggestive, but the share owed to the language is not yet measured.</p>
</div>

A prover can only use what the code makes visible. In Nikaia a function
declares whether it keeps, changes or only looks at each argument, and the
compiler refuses a change to a value while a view of it is still in use
([ADR-views-3](/docs/specification/adr/adr-views-3.html)). Lengths, guards and types are facts the
prover reads directly. The working assumption is that a language designed for
this needs a smaller prover, and fewer annotations, for the same reach. The
question is how much of the reach measured above comes from the language and
how much from the solver.

### Is it predictable for someone who does not know the prover?

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

### Can it reason about what a list holds?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 35 % held back for what is not yet known"><span class="rq-known" style="width: 65%"><span class="rq-done" style="width: 8%"></span></span><span class="rq-unknown" style="width: 35%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">None of the e-graph, the theory of arrays or the rules over every element is built yet, and there is no evidence either way.</p>
</div>

The solver today decides linear integer arithmetic. Many arguments about
programs need more. From `i == c` it should follow that `xs[i] == xs[c]`, and
after `xs[i] = v` a read at `k` should give `v` when `k == i` and the old
value otherwise. Equality with uninterpreted functions, built as an e-graph,
and a theory of arrays carry exactly this. The procedure is decided
([ADR-270](/docs/specification/adr/adr-270.html) D13) and not yet built. Beyond it lie claims
about every element, such as a sorted list or a partition, which need a rule
over all elements ([#798](https://github.com/Nikaia-Foundation/Nikaia/issues/798), open for decision) and invariants over
list contents that the compiler would have to find by itself. Finding such
invariants without help is the hardest part of the question "nothing written
for it", and the place where its limits are most likely to show.

### What does trust cost in an ordinary build?

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

### What can outside help add?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">The checked loop problem a proposal would be checked against is built, and taking proposals from outside is not yet. There is no evidence yet.</p>
</div>

A loop invariant may be proposed by any program, a language model among
them, and a proposal is used only after the compiler has checked it
([ADR-361](/docs/specification/adr/adr-361.html) D6). Much of Nikaia's own code is written with
the help of such models.

The interface is decided and not yet built. A proposer is a program named in
the manifest, and only `nikaia prove --propose` starts it, never `build`,
`run` or `test`. It reads one line of JSON per loop the build left open, with
the problem in SMT-LIB, the loop's source and the names it reads, and answers
with candidate invariants. What passes the check is written to
`nikaia.proofs`, so the next build uses it without asking anyone, and a wrong
proposal costs time and nothing else. Which model, provider or prompt is used
lives in the proposer and not in the compiler, and the program's code leaves
the machine only through that command and that entry.

The question is how much a proposer adds to what
the compiler finds alone, measured by what is proved with it and without it,
and whether this keeps the cost of a proof off the person who writes the
program.

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

## Ideas this rests on

| Idea | Recorded in |
| :--- | :--- |
| An `assert` is a contract, proved where possible and a refused program where not | [ADR-269](/docs/specification/adr/adr-269.html), [ADR-380](/docs/specification/adr/adr-380.html) |
| A check is dropped only where a proof shows it cannot fail | [ADR-306](/docs/specification/adr/adr-306.html) |
| What a grammar matched and what a callee promises are facts | [ADR-314](/docs/specification/adr/adr-314.html), [ADR-359](/docs/specification/adr/adr-359.html) |
| A type's rule about its fields is proved where values are built and known where they are read | [ADR-369](/docs/specification/adr/adr-369.html) |
| A pure `bool` call is a property the prover knows by name | [ADR-381](/docs/specification/adr/adr-381.html) |
| A loop is a problem in Horn clauses, and invariants are found, checked, and may be proposed from outside | [ADR-361](/docs/specification/adr/adr-361.html) |
| A function says what it keeps and changes, and a view of a value forbids changing it | [ADR-views-3](/docs/specification/adr/adr-views-3.html) |
| The solver is written in Nikaia, untrusted, its proofs checked and committed | [ADR-270](/docs/specification/adr/adr-270.html) |
| A strategy layer switches engines during a search, and effort is counted in work, not time | [ADR-270](/docs/specification/adr/adr-270.html) D14, D25, [ADR-306](/docs/specification/adr/adr-306.html) |
| A library promises properties its callers use as facts | [ADR-381](/docs/specification/adr/adr-381.html), [#798](https://github.com/Nikaia-Foundation/Nikaia/issues/798) |
