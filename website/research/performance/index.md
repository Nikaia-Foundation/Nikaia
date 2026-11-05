# Open questions about performance with safety

For now, Nikaia keeps an index or overflow check wherever the compiler has
not proved that the check cannot fail, and a proved check is left out only
where a build asks for it ([ADR-306](/docs/specification/adr/adr-306.html)). A proof can do more
than remove a check. A loop without a check on each step can be vectorised, a
sum that cannot overflow can be split across vector lanes, and a number whose
range is known can be held in a narrower type. These questions ask how far
proofs carry a program towards the speed of the fastest code while it stays
safe, what it takes to trust a proof enough to act on it, and what a compiler
built around such facts can do in compile time and run time.

They build on the questions about the prover, since a fact the prover cannot
show is a fact no optimisation can use. Those are under
[open questions about the prover](../prover/), which also explains the two
bars under each question.

## Where it stands

The [1BRC study](/fast/against-tuned-rust.html) counts Nikaia's programs for
the One Billion Row Challenge against Rust written by people and by an AI
assistant, in instructions per row on `x86-64`. Its hand-tuned program
`fast.nika` needs 143.4 instructions a row with every check kept and no
`unsafe` in the emitted Rust, and 95.0 with the checks the compiler proved
written without them. The fastest Rust program by a person in the study needs
131.3, and the fastest safe one 152.2. These are counts for one program on
one machine level, and the figures with proved checks removed rest on
trusting the proofs. The study gives the full table, the method and the open
items.

## The questions

### Can safe code reach the speed of the fastest unsafe code?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 35 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 47%"></span></span><span class="rq-unknown" style="width: 25%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 20%"></span></div></div>
<p class="rq-note">The proofs, the setting that writes a proved index or operation without its check, and part of the vector API are built. One program, <code>fast.nika</code>, needs 95.0 instructions a row with proved checks dropped against 131.3 for the fastest Rust by a person, and 143.4 with every check kept, which is still fewer than the 152.2 of the fastest safe Rust.</p>
</div>

The study's programs show where the time goes. In `fast.nika` the checks the
compiler has proved account for the step from 143.4 to 95.0 instructions a
row. The rest of the gap to `fast-unsafe.nika`, which believes its claims
about the input instead of checking them, is what the prover still has to
learn, among it the sums and counts of every slot proved over the whole fold
([#770](https://github.com/Nikaia-Foundation/Nikaia/issues/770)). The question is whether this holds beyond one benchmark,
whether programs written plainly and the compiler's own code reach the same
league once their checks are proved, and how much of a program's speed comes
from proofs, from the language's other guarantees, and from the backend.

### Would we act on a proof, and how is that trust earned?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 20 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 29%"></span></span><span class="rq-unknown" style="width: 30%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">Certificates checked before a proof is used, and a default that keeps every check, are built. There is no evidence yet that would justify dropping proved checks by default.</p>
</div>

A proof is checked by its certificate before it is used, but the certificate
covers the solver's arithmetic and not whether the question the compiler
asked says what the program does. A defect there, or in the lowering, would
turn a program that stops cleanly at a failed check into one that reads past
the end of a list. That is why dropping proved checks is off by default and
has to be asked for ([ADR-306](/docs/specification/adr/adr-306.html)). Having proved every check
is therefore not the same as being willing to remove them.

The question is what it takes to make removing them the safe choice, and how
that is shown. Several steps would each add trust and can each be measured.
The certificate checker itself can be proved free of run-time errors
([#1104](https://github.com/Nikaia-Foundation/Nikaia/issues/1104)). Every proof can also be checked by a checker this
project did not write, through the Alethe export
([ADR-270](/docs/specification/adr/adr-270.html) D6, D8). The same program can be built with and
without the checks and compared on the whole test suite and on generated
inputs. Further off are a check per build that the question asked matches
the program, and a formal meaning of the language to check it against
([formal verification](/docs/formal-verification.html), items 9 to 14). The
measure is how many defects each step finds, and which steps together leave
no known way for a wrong proof to reach a program.

### Which proofs does each optimisation need?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 25 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 36%"></span></span><span class="rq-unknown" style="width: 30%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 10%"></span></div></div>
<p class="rq-note">Dropping a proved index or overflow check is built. In the 1BRC study the proved checks are worth 48 instructions a row, and in the solver's kernels most of the gap to hand-written Rust was the conversion of an index the prover can show is not negative.</p>
</div>

Several optimisations wait for a fact before they are allowed. A loop with a
check on every step has a way out on every step, which usually keeps it from
being vectorised. A sum may be split across vector lanes only when it cannot
overflow. A number proved to stay in a smaller range can be held in a
narrower type, so twice as many fit in a vector register. Two lists that
cannot overlap let reads and writes be reordered, and in Nikaia that fact
comes from the language rather than the prover, because a function declares
what it keeps and changes and a value may not change while a view of it is
in use ([ADR-views-3](/docs/specification/adr/adr-views-3.html)). Bounds on a loop let checks move
out of it ([ADR-361](/docs/specification/adr/adr-361.html)). In the solver's own kernels,
converting an index with a check for a negative value cost the big-number
kernel a factor of 3.3 against hand-written Rust
([solver workload](/docs/solver-workload.html), section 8).

The question is which facts open which optimisations, how much each one is
worth, and so which of the prover's open questions matter most for speed.

### Can our own optimiser over a fast backend beat every combination?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 2 % of the estimated whole, 35 % held back for what is not yet known"><span class="rq-known" style="width: 65%"><span class="rq-done" style="width: 3%"></span></span><span class="rq-unknown" style="width: 35%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">Nothing is built yet, and the first step is to measure where each combination stands. There is no evidence yet.</p>
</div>

Today the compiler writes Rust, and `rustc` with LLVM turns it into machine
code. LLVM spends much of its time finding out facts again that the compiler
already has as proofs, such as whether an index is in range, whether two
references overlap, whether a call touches anything and whether an addition
can overflow. A backend such as Cranelift is fast because it optimises
little, so whatever the compiler hands it, it gains in full. The idea in
[#1235](https://github.com/Nikaia-Foundation/Nikaia/issues/1235), filed for the future, is that the optimisations that matter
are done once, by Nikaia's own optimiser, from facts it already has, and a
fast backend writes the machine code. The aim is to be faster in compile time
and in run time than Nikaia over LLVM, Rust over LLVM and Rust over
Cranelift.

The hardest part is run time against Nikaia over LLVM, which would get the
same facts and still add its own passes, such as inlining, vectorisation and
loop transformations. Beating it means covering those and adding what only
Nikaia can know, such as merging calls of pure functions or fusing chains
whose steps are proved independent. The first steps are measurements of
where each combination stands today and of how much of `rustc`'s time the
bodies of Nikaia's functions cost.

## Ideas this rests on

| Idea | Recorded in |
| :--- | :--- |
| A check is dropped only where a proof shows it cannot fail, and dropping is asked for | [ADR-306](/docs/specification/adr/adr-306.html) |
| Every proof is checked by a certificate, and Alethe lets a checker from outside check it too | [ADR-270](/docs/specification/adr/adr-270.html) D5, D6 |
| A claim about the input may be believed with `assume`, visibly, and is checked in the default build | [ADR-376](/docs/specification/adr/adr-376.html) |
| A function declares what it keeps and changes, and a value does not change under a view | [ADR-views-3](/docs/specification/adr/adr-views-3.html) |
| Performance is counted in instructions against tuned Rust and not timed | [measuring](/docs/measuring.html) |
| Our own optimiser from the facts we have, and a fast backend behind it | [#1235](https://github.com/Nikaia-Foundation/Nikaia/issues/1235) |
