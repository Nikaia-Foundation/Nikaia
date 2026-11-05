# How the prover is built

This page describes the prover's internal structure, what exists today, and
what is decided and not yet built. The decisions are in
[ADR-270](/docs/specification/adr/adr-270.html) for the solver and
[ADR-361](/docs/specification/adr/adr-361.html) for loops, and the numbers
that shaped them are in the [solver workload](/docs/solver-workload.html).
The questions this structure serves are under
[open questions about the prover](./).

## Three layers

The prover is three layers, and only the top one knows Nikaia
([ADR-270](/docs/specification/adr/adr-270.html) D1).

<figure class="nkd">
<svg viewBox="0 0 760 410" role="img" aria-labelledby="nkd-layers-title">
<title id="nkd-layers-title">The prover's three layers. The frontend asks a query; a solver, which is not trusted, answers with a certificate; the checker replays it; what holds goes into nikaia.proofs and back to the frontend, which drops the check.</title>
<g class="cy"><rect class="box" x="40" y="16" width="680" height="74" rx="10"/>
<text class="t" x="60" y="42">frontend</text>
<text class="m" x="60" y="62">part of the compiler: walks the program, collects facts, asks questions,</text>
<text class="m" x="60" y="79">and turns each answer into a dropped check or a message</text></g>
<g class="vi"><rect class="box" x="40" y="120" width="680" height="170" rx="12"/>
<text class="t" x="60" y="268">logic layer</text>
<text class="m" x="700" y="268" text-anchor="end">no Nikaia syntax, usable by any package</text></g>
<g class="cy"><rect class="box" x="60" y="136" width="190" height="90" rx="8"/>
<text class="t" x="76" y="162">query</text>
<text class="m" x="76" y="184">facts, a goal and</text>
<text class="m" x="76" y="201">opaque ids</text></g>
<g class="gr glow"><rect class="box" x="285" y="136" width="190" height="90" rx="8"/>
<text class="t" x="301" y="162">checker</text>
<text class="m" x="301" y="184">replays every</text>
<text class="m" x="301" y="201">certificate; trusted</text></g>
<g class="am"><rect class="box" x="510" y="136" width="190" height="90" rx="8"/>
<text class="t" x="526" y="162">nikaia.proofs</text>
<text class="m" x="526" y="184">one answer per</text>
<text class="m" x="526" y="201">question, committed</text></g>
<g class="pk dash"><rect class="box" x="40" y="330" width="680" height="64" rx="10"/>
<text class="t" x="60" y="357">solvers</text>
<text class="m" x="60" y="378">search for a proof or a counterexample</text>
<text class="t" x="700" y="357" text-anchor="end">never trusted</text></g>
<g class="cy"><path class="flow" d="M155 90 V128"/><polygon class="head" points="150,128 160,128 155,136"/>
<circle class="dot" r="3.5"><animateMotion dur="1.6s" repeatCount="indefinite" path="M155 90 V136"/></circle>
<text class="l" x="165" y="112">asks</text></g>
<g class="cy"><path class="flow" d="M225 226 V322"/><polygon class="head" points="220,322 230,322 225,330"/>
<circle class="dot" r="3.5"><animateMotion dur="1.6s" repeatCount="indefinite" path="M225 226 V330"/></circle>
<text class="l" x="235" y="314">query</text></g>
<g class="pk"><path class="flow" d="M380 330 V234"/><polygon class="head" points="375,234 385,234 380,226"/>
<circle class="dot" r="3.5"><animateMotion dur="1.6s" repeatCount="indefinite" path="M380 330 V226"/></circle>
<text class="l" x="390" y="314">certificate</text></g>
<g class="gr"><path class="flow" d="M475 181 H502"/><polygon class="head" points="502,176 502,186 510,181"/>
<circle class="dot" r="3.5"><animateMotion dur="0.8s" repeatCount="indefinite" path="M475 181 H510"/></circle></g>
<g class="am"><path class="flow" d="M605 136 V98"/><polygon class="head" points="600,98 610,98 605,90"/>
<circle class="dot" r="3.5"><animateMotion dur="1.6s" repeatCount="indefinite" path="M605 136 V90"/></circle>
<text class="l" x="615" y="112">check dropped</text></g>
</svg>
<figcaption>Only the checker is trusted. A solver's answer counts once the checker has replayed its certificate, and what holds is committed to <code>nikaia.proofs</code> (ADR-270 D1, D5, D19).</figcaption>
</figure>

The frontend is part of the compiler (`prover_walk`, `prover_claims`,
`prover_loops` and their neighbours in `crates/nikaia-std/src/tools/`). It
reads comparisons, guards, lengths, what a callee promises and a type's rule,
and asks each question as a self-contained query made of facts, a goal, and
opaque ids for what they stand for (D3, D7). An answer is one of three. It is
proved, with a certificate, or refuted, with values that are checked by
evaluating them, or unknown.

The logic layer holds the terms, the normal form a query is keyed by, the
checker, and SMT-LIB and Alethe at its edge for comparing with other solvers
(D6, D19). Loops reach it as problems in constrained Horn clauses, which state
what holds when the loop is reached, one turn as a relation, each way out,
and what must hold after it ([ADR-361](/docs/specification/adr/adr-361.html)
D1).

## What a build trusts

No solver is trusted. A proof is used only after the checker has replayed its
certificate step by step, with clause steps, combinations of inequalities and
their integer rounding, case splits, and the definition of a quotient by a
constant (D5). The checker replays and does not search, which keeps it small.

What a build uses is the committed file `nikaia.proofs`, with one line per
question, keyed by the question's normal form, holding its answer and the
evidence. Every entry is checked when it is read. A normal build searches only
for what is missing, and `--locked` searches nothing and fails on any
difference (D19-D22). The search itself may differ between machines and runs,
and what one machine proved is not rejected by another (D4).

## The solver, today and as decided

Today's solver is the reference solver, which decides linear integer
arithmetic by Fourier-Motzkin elimination with case splits on demand and is
written in Nikaia (`logic_lia`, `logic_search`). Its successor is decided
piece by piece, and each step is measured before the next (D8).

<figure class="nkd">
<svg viewBox="0 0 760 420" role="img" aria-labelledby="nkd-propose-title">
<title id="nkd-propose-title">nikaia prove --propose hands each open loop to a proposer, any program the manifest names; the proposer answers with candidate invariants; the checker keeps only what it can check; that goes into nikaia.proofs, which every build reads without calling a proposer.</title>
<g class="cy"><rect class="box" x="230" y="16" width="300" height="58" rx="10"/>
<text class="t" x="250" y="42">nikaia prove --propose</text>
<text class="m" x="250" y="62">the only command that starts one</text></g>
<g class="pk dash"><rect class="box" x="230" y="130" width="300" height="70" rx="10"/>
<text class="t" x="250" y="156">proposer</text>
<text class="m" x="250" y="176">any program the manifest names:</text>
<text class="m" x="250" y="192">a language model, a learner from runs</text></g>
<text class="l" x="20" y="160">outside the compiler,</text>
<text class="l" x="20" y="177">never trusted</text>
<g class="gr glow"><rect class="box" x="230" y="256" width="300" height="58" rx="10"/>
<text class="t" x="250" y="282">checker</text>
<text class="m" x="250" y="302">keeps only what it can check</text></g>
<g class="am"><rect class="box" x="230" y="356" width="300" height="50" rx="10"/>
<text class="t" x="250" y="378">nikaia.proofs</text>
<text class="m" x="250" y="396">committed with the code</text></g>
<g class="sl"><rect class="box" x="20" y="356" width="170" height="50" rx="10"/>
<text class="t" x="36" y="378">nikaia build</text>
<text class="m" x="36" y="396">calls no proposer</text></g>
<g class="cy"><path class="flow" d="M380 74 V122"/><polygon class="head" points="375,122 385,122 380,130"/>
<circle class="dot" r="3.5"><animateMotion dur="1.6s" repeatCount="indefinite" path="M380 74 V130"/></circle>
<text class="l" x="392" y="106">one JSON line per open loop</text></g>
<g class="pk"><path class="flow" d="M380 200 V248"/><polygon class="head" points="375,248 385,248 380,256"/>
<circle class="dot" r="3.5"><animateMotion dur="1.6s" repeatCount="indefinite" path="M380 200 V256"/></circle>
<text class="l" x="392" y="232">candidate invariants</text></g>
<g class="gr"><path class="flow" d="M380 314 V348"/><polygon class="head" points="375,348 385,348 380,356"/>
<circle class="dot" r="3.5"><animateMotion dur="1.2s" repeatCount="indefinite" path="M380 314 V356"/></circle>
<text class="l" x="392" y="339">what holds</text></g>
<g class="am"><path class="flow" d="M230 381 H198"/><polygon class="head" points="198,376 198,386 190,381"/>
<circle class="dot" r="3.5"><animateMotion dur="1s" repeatCount="indefinite" path="M230 381 H190"/></circle></g>
</svg>
<figcaption>A proposal is checked like one of the compiler's own answers, so a wrong one costs time and nothing else (ADR-361 D6). Decided, not yet built.</figcaption>
</figure>

The skeleton is a CDCL loop over Boolean literals, and every theory sits
behind one interface. A theory takes a literal, propagates what it implies,
checks consistency, explains a propagation, backtracks, gives a model, and
names the terms it shares (D12). The compiler's own questions are tiny, with
at most four variables and mostly bounds on single variables or differences
between two, so fast paths answer those without entering the general search
and without allocating (D9, D12). Theories are combined through the e-graph
by model-based combination. A theory's model proposes equalities between
shared terms, the search decides them, and a theory that disagrees answers
with a conflict (D13). Numbers start as `i64` and fall back to big integers
and fractions only where a value outgrows 32 bits (D11). Clauses live in one
arena of `u32` words with watch lists that hold a blocking literal (D16).

| Part | Decided in | State |
| :--- | :--- | :--- |
| Three layers, query and answer, certificates and their checker | D1-D5 | built, in Nikaia |
| SMT-LIB and Alethe at the edge | D6 | built |
| `nikaia.proofs`, checked on read, `--locked` | D19-D22 | built |
| Loops as Horn clauses, invariants by closed forms and intervals, each checked | [ADR-361](/docs/specification/adr/adr-361.html) | built for whole numbers, more strategies to come (D5 there) |
| The solver's inner loops measured in Nikaia against Rust | D8 step 1 | measured, the gaps found are in the lowering of indexes ([solver workload](/docs/solver-workload.html), section 8) |
| Reference solver moved from Rust into Nikaia | D8 step 2 | done |
| `u32` variables, sparse rows, big numbers, theory interface | D8 step 3, D10-D12 | not built |
| CDCL(T) skeleton, simplex, fast paths | D8 step 5, D12 | not built |
| Cuts, local search, strategy layer | D8 step 6, D14 | not built |
| E-graph, combination, arrays, bit-vectors | D13 | decided, not built |
| Parallel search within one question | D8 step 7, D18 | not built |

The same parts, each with a bar for how far it is built, are under
[what is being built](./#what-is-being-built).

## Invariants from a language model or another program

A solver inside the compiler is not the only place a proof can come from. An
outside program, a language model among them, may propose a loop invariant,
and the compiler uses it only after checking it like one of its own
([ADR-361](/docs/specification/adr/adr-361.html) D6).

<figure class="nkd">
<svg viewBox="0 0 760 460" role="img" aria-labelledby="nkd-solver-title">
<title id="nkd-solver-title">The solver as decided: a strategy layer above a CDCL(T) skeleton, which drives fast paths, linear arithmetic and an e-graph; arrays and bit-vectors sit behind the e-graph. Today the reference solver, Fourier-Motzkin with case splits written in Nikaia, is built; the rest is decided and not yet built.</title>
<g class="vi dash"><rect class="box" x="20" y="18" width="26" height="14" rx="3"/></g>
<text class="l" x="54" y="30">decided, not yet built</text>
<g class="gr"><rect class="box" x="20" y="42" width="26" height="14" rx="3"/></g>
<text class="l" x="54" y="54">built</text>
<g class="am dash"><rect class="box" x="230" y="16" width="300" height="58" rx="10"/>
<text class="t" x="250" y="42">strategy layer · D14</text>
<text class="m" x="250" y="62">picks engines, counts effort</text></g>
<g class="vi dash glow"><rect class="box" x="150" y="110" width="460" height="72" rx="10"/>
<text class="t" x="170" y="136">CDCL(T) skeleton · D12</text>
<text class="m" x="170" y="156">decisions, propagation, conflict analysis,</text>
<text class="m" x="170" y="173">backjumping, restarts, learned clauses</text></g>
<g class="cy dash"><rect class="box" x="20" y="230" width="220" height="76" rx="10"/>
<text class="t" x="36" y="256">fast paths · D12</text>
<text class="m" x="36" y="276">intervals, differences,</text>
<text class="m" x="36" y="293">no general search</text></g>
<g class="cy dash"><rect class="box" x="270" y="230" width="220" height="76" rx="10"/>
<text class="t" x="286" y="256">linear arithmetic</text>
<text class="m" x="286" y="276">simplex over the</text>
<text class="m" x="286" y="293">integers, cuts · D5, D11</text></g>
<g class="vi dash"><rect class="box" x="520" y="230" width="220" height="76" rx="10"/>
<text class="t" x="536" y="256">e-graph · D13</text>
<text class="m" x="536" y="276">equality, where the</text>
<text class="m" x="536" y="293">theories meet</text></g>
<g class="cy dash"><rect class="box" x="385" y="370" width="165" height="64" rx="10"/>
<text class="t" x="401" y="396">arrays</text>
<text class="m" x="401" y="416">lemmas on demand</text></g>
<g class="cy dash"><rect class="box" x="565" y="370" width="180" height="64" rx="10"/>
<text class="t" x="581" y="396">bit-vectors</text>
<text class="m" x="581" y="416">blasted into clauses</text></g>
<g class="gr glow"><rect class="box" x="20" y="360" width="340" height="80" rx="10"/>
<text class="t" x="36" y="386">today: reference solver</text>
<text class="m" x="36" y="408">Fourier-Motzkin with case splits,</text>
<text class="m" x="36" y="425">written in Nikaia</text></g>
<g class="am"><path class="flow" d="M380 74 V102"/><polygon class="head" points="375,102 385,102 380,110"/>
<circle class="dot" r="3.5"><animateMotion dur="1.2s" repeatCount="indefinite" path="M380 74 V110"/></circle></g>
<g class="vi"><path class="flow" d="M300 182 V206 H130 V222"/><polygon class="head" points="125,222 135,222 130,230"/>
<circle class="dot" r="3.5"><animateMotion dur="2s" repeatCount="indefinite" path="M300 182 V206 H130 V230"/></circle></g>
<g class="vi"><path class="flow" d="M380 182 V222"/><polygon class="head" points="375,222 385,222 380,230"/>
<circle class="dot" r="3.5"><animateMotion dur="1.2s" repeatCount="indefinite" path="M380 182 V230"/></circle></g>
<g class="vi"><path class="flow" d="M460 182 V206 H630 V222"/><polygon class="head" points="625,222 635,222 630,230"/>
<circle class="dot" r="3.5"><animateMotion dur="2s" repeatCount="indefinite" path="M460 182 V206 H630 V230"/></circle></g>
<g class="vi"><path class="flow" d="M590 306 V336 H467 V362"/><polygon class="head" points="462,362 472,362 467,370"/>
<circle class="dot" r="3.5"><animateMotion dur="1.8s" repeatCount="indefinite" path="M590 306 V336 H467 V370"/></circle></g>
<g class="vi"><path class="flow" d="M670 306 V362"/><polygon class="head" points="665,362 675,362 670,370"/>
<circle class="dot" r="3.5"><animateMotion dur="1.4s" repeatCount="indefinite" path="M670 306 V370"/></circle></g>
</svg>
<figcaption>Every theory sits behind one interface of the skeleton, and theories meet through the e-graph by model-based combination (ADR-270 D12, D13).</figcaption>
</figure>

Only `nikaia prove --propose` starts a proposer, never `build`, `run` or
`test`, so the program's code leaves the machine only through that command
and the proposer the manifest names. A wrong proposal costs time and nothing
else. The proposer is a child process, as language servers are, so what a
provider, a model, a key or a prompt is stays out of the compiler. This is
decided and not yet built, and what it adds is asked under
[open questions about the prover](./#how-much-can-large-language-models-help-the-prover).

## What the language has to offer the solver

The solver is written in Nikaia, so building it is also a test of the
language, with arenas of `u32`, flat loops over lists, and in-place changes
to one field of the solver while another is read. A probe of the forms a
CDCL(T) core and an e-graph need found most of them working and a few
defects in how they are lowered, each filed with its probe. They are
[#1206](https://github.com/Nikaia-Foundation/Nikaia/issues/1206) (an argument
that reads the list whose element a call changes),
[#1207](https://github.com/Nikaia-Foundation/Nikaia/issues/1207) (the value
`or_insert` hands back),
[#1208](https://github.com/Nikaia-Foundation/Nikaia/issues/1208) (a part of
`mut self` handed to a `keep` parameter),
[#1209](https://github.com/Nikaia-Foundation/Nikaia/issues/1209) (a part of a
list handed to a function and changed there),
[#907](https://github.com/Nikaia-Foundation/Nikaia/issues/907) (`contains`
with a computed argument) and
[#591](https://github.com/Nikaia-Foundation/Nikaia/issues/591) (a declared
struct as the key of a map). The work on the solver waits for these rather
than writing around them.
