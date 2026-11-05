# Research questions

Some of what Nikaia sets out to do has not been done in this form before, or
not at this scale, and whether it works is something to find out rather than
to promise. This section collects those questions. Each one says what we want
to find out, how the answer will be measured, where the work stands, and which
decision records and issues it rests on.

A question here is open. Where a page quotes a figure, the figure was measured
on a named commit and the page links to where it was measured. Where it
describes something that is decided but not yet built, it says so.

## Areas

[The prover](prover/) is the part of the compiler that proves claims about a
program while it is built: that an index stays inside its list, that a sum
does not overflow, that an `assert` holds. Its questions are about how far
this reaches without anything written for it, what is left for the
programmer to write, and how the prover itself has to be built to carry it.
[How the prover is built](prover/architecture.html) describes its layers and
which of them exist today.

<div class="rq-over">
<div class="rq-over-row rq-over-head"><span>Question</span><span>Built</span><span>Evidence</span></div>
<div class="rq-over-row"><a href="prover/#how-far-does-it-reach-with-nothing-written-for-it">How far does it reach with nothing written for it?</a><div class="rq-build" role="img" aria-label="Built: about 35 % of the estimated whole; 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 47%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 15%"></span></div></div>
<div class="rq-over-row"><a href="prover/#can-an-unproved-check-one-day-refuse-the-program">Can an unproved check one day refuse the program?</a><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole; 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="prover/#how-much-does-the-design-of-the-language-carry">How much does the design of the language carry?</a><div class="rq-build" role="img" aria-label="Built: about 30 % of the estimated whole; 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 40%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
<div class="rq-over-row"><a href="prover/#is-it-predictable-for-someone-who-does-not-know-the-prover">Is it predictable for someone who does not know the prover?</a><div class="rq-build" role="img" aria-label="Built: about 45 % of the estimated whole; 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 60%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
<div class="rq-over-row"><a href="prover/#can-it-reason-about-what-a-list-holds">Can it reason about what a list holds?</a><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole; 35 % held back for what is not yet known"><span class="rq-known" style="width: 65%"><span class="rq-done" style="width: 8%"></span></span><span class="rq-unknown" style="width: 35%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="prover/#what-does-trust-cost-in-an-ordinary-build">What does trust cost in an ordinary build?</a><div class="rq-build" role="img" aria-label="Built: about 30 % of the estimated whole; 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 40%"></span></span><span class="rq-unknown" style="width: 25%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 20%"></span></div></div>
<div class="rq-over-row"><a href="prover/#what-can-outside-help-add">What can outside help add?</a><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole; 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="prover/#where-should-the-prover-spend-its-effort">Where should the prover spend its effort?</a><div class="rq-build" role="img" aria-label="Built: about 15 % of the estimated whole; 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 21%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<div class="rq-over-row"><a href="prover/#can-the-standard-library-hand-out-proved-facts">Can the standard library hand out proved facts?</a><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole; 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
</div>

The two bars are explained on [the prover's page](prover/#where-it-stands).
