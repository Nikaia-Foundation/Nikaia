# Research questions

Some of what Nikaia sets out to do has not been done in this form before, or
not at this scale, and whether it works is something to find out rather than
to promise. This section collects those questions. Each one says what we want
to find out, how the answer will be measured, where the work stands, and
which decision records and issues it rests on.

A question here is open. Where a page quotes a figure, the figure was
measured and the page links to where. Where it describes something that is
decided but not yet built, it says so.

## Areas

[The prover](prover/) is the part of the compiler that proves claims about a
program while it is built, such as that an index stays inside its list, that
a sum does not overflow, or that an `assert` holds. Its questions are about
how much the prover proves without invariants or other hints written by the programmer, what is left for the
programmer to write, and how the prover itself has to be built to carry it.
[How the prover is built](prover/architecture.html) describes its layers and
which of them exist today.

<div class="rq-over">
<div class="rq-over-row rq-over-head"><span>Question</span><span>Built</span><span>Evidence</span></div>
<div class="rq-over-row"><a href="prover/#does-nikaia-find-every-invariant-by-itself-with-none-written-by-hand">Does Nikaia find every invariant by itself, with none written by hand?</a><div class="rq-build" role="img" aria-label="Built: about 35 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 47%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 15%"></span></div></div>
<div class="rq-over-row"><a href="prover/#can-the-compiler-one-day-refuse-a-program-whose-index-or-overflow-checks-are-not-all-proved">Can the compiler one day refuse a program whose index or overflow checks are not all proved?</a><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="prover/#how-much-of-the-provers-reach-comes-from-the-design-of-nikaia">How much of the prover's reach comes from the design of Nikaia?</a><div class="rq-build" role="img" aria-label="Built: about 30 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 40%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
<div class="rq-over-row"><a href="prover/#can-a-programmer-who-does-not-know-the-prover-predict-which-proofs-hold">Can a programmer who does not know the prover predict which proofs hold?</a><div class="rq-build" role="img" aria-label="Built: about 45 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 60%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
<div class="rq-over-row"><a href="prover/#can-the-prover-reason-about-what-a-list-holds">Can the prover reason about what a list holds?</a><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 35 % held back for what is not yet known"><span class="rq-known" style="width: 65%"><span class="rq-done" style="width: 8%"></span></span><span class="rq-unknown" style="width: 35%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="prover/#can-the-prover-use-rules-over-every-element-of-a-list-and-keep-its-proofs-stable">Can the prover use rules over every element of a list and keep its proofs stable?</a><div class="rq-build" role="img" aria-label="Built: about 8 % of the estimated whole, 35 % held back for what is not yet known"><span class="rq-known" style="width: 65%"><span class="rq-done" style="width: 12%"></span></span><span class="rq-unknown" style="width: 35%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="prover/#what-does-checking-every-proof-cost-in-an-ordinary-build">What does checking every proof cost in an ordinary build?</a><div class="rq-build" role="img" aria-label="Built: about 30 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 40%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 20%"></span></div></div>
<div class="rq-over-row"><a href="prover/#how-much-can-large-language-models-help-the-prover">How much can large language models help the prover?</a><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="prover/#where-should-the-prover-spend-its-effort">Where should the prover spend its effort?</a><div class="rq-build" role="img" aria-label="Built: about 15 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 21%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="prover/#can-the-standard-library-hand-out-proved-facts">Can the standard library hand out proved facts?</a><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
</div>

[Performance with safety](performance/) asks how far proofs carry a program
towards the speed of the fastest code while it stays safe. Its questions are
about which proofs each optimisation needs, what it takes to trust a proof
enough to leave a check out, and whether an optimiser of our own over a fast
backend can beat the established ones in compile time and run time. It builds
on the questions about the prover, and its starting point is the
[1BRC study](/fast/against-tuned-rust.html).

<div class="rq-over">
<div class="rq-over-row rq-over-head"><span>Question</span><span>Built</span><span>Evidence</span></div>
<div class="rq-over-row"><a href="performance/#can-safe-code-reach-the-speed-of-the-fastest-unsafe-code">Can safe code reach the speed of the fastest unsafe code?</a><div class="rq-build" role="img" aria-label="Built: about 35 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 47%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 20%"></span></div></div>
<div class="rq-over-row"><a href="performance/#should-the-compiler-drop-a-check-because-a-proof-says-it-cannot-fail-and-how-does-a-proof-earn-that-trust">Should the compiler drop a check because a proof says it cannot fail, and how does a proof earn that trust?</a><div class="rq-build" role="img" aria-label="Built: about 20 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 29%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="performance/#which-proofs-does-each-optimisation-need">Which proofs does each optimisation need?</a><div class="rq-build" role="img" aria-label="Built: about 25 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 36%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 10%"></span></div></div>
<div class="rq-over-row"><a href="performance/#can-nikaias-own-optimiser-over-a-fast-backend-beat-every-combination-of-existing-optimisers-and-backends">Can Nikaia's own optimiser over a fast backend beat every combination of existing optimisers and backends?</a><div class="rq-build" role="img" aria-label="Built: about 2 % of the estimated whole, 35 % held back for what is not yet known"><span class="rq-known" style="width: 65%"><span class="rq-done" style="width: 3%"></span></span><span class="rq-unknown" style="width: 35%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
</div>

[Libraries written in Nikaia](libraries/) asks whether a library written in
Nikaia can be both safe and fast, with every index and operation in it proved to
stay inside its bounds, and its instructions counted against a tuned library
in Rust. Its first case study is the `yaml` package, measured against
`saphyr-parser`. Its questions are about the proofs, the distance to the tuned
parser, whether the compiler can show where to optimise, a grammar as the
parser, keeping comments when a file is written back, and an interface its
users already know.

<div class="rq-over">
<div class="rq-over-row rq-over-head"><span>Question</span><span>Built</span><span>Evidence</span></div>
<div class="rq-over-row"><a href="libraries/#is-every-bound-check-in-the-yaml-package-proved">Is every bound check in the yaml package proved?</a><div class="rq-build" role="img" aria-label="Built: about 70 % of the estimated whole, 20 % held back for what is not yet known"><span class="rq-known" style="width: 80%"><span class="rq-done" style="width: 88%"></span></span><span class="rq-unknown" style="width: 20%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 25%"></span></div></div>
<div class="rq-over-row"><a href="libraries/#how-close-does-the-yaml-package-come-to-the-speed-of-saphyr-parser">How close does the yaml package come to the speed of saphyr-parser?</a><div class="rq-build" role="img" aria-label="Built: about 35 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 50%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
<div class="rq-over-row"><a href="libraries/#does-the-compiler-show-where-to-optimise">Does the compiler show where to optimise?</a><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="libraries/#can-a-grammar-be-the-yaml-packages-parser">Can a grammar be the yaml package's parser?</a><div class="rq-build" role="img" aria-label="Built: about 60 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 80%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 20%"></span></div></div>
<div class="rq-over-row"><a href="libraries/#can-the-yaml-package-keep-comments-and-layout-when-it-writes-a-file-back">Can the yaml package keep comments and layout when it writes a file back?</a><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="libraries/#does-the-yaml-package-feel-familiar-to-people-who-know-the-yaml-libraries-of-rust">Does the yaml package feel familiar to people who know the YAML libraries of Rust?</a><div class="rq-build" role="img" aria-label="Built: about 70 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 93%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 15%"></span></div></div>
</div>

The two bars are explained under
[open questions about the prover](prover/#where-the-prover-stands-today).
