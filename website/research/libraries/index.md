# Libraries written in Nikaia

A language shows what it can carry when real libraries are written in it,
libraries that other programs depend on, that read input nobody has checked,
and that are measured against the best code in their field. These questions
ask whether a library written in Nikaia can be both safe and fast. Safe here
means that the compiler proves every index and every arithmetic operation in
the library stays inside its bounds, rather than checking it while the library
runs. Fast
means that the library is counted in instructions against a tuned library
written in Rust that does the same work.

The questions build on those about the prover and about performance with
safety, since a library is where both meet a real program. They are under
[open questions about the prover](../prover/), which also explains the two
bars under each question, and [performance with safety](../performance/).

## The yaml package, the first case study

The first library is the official
[`yaml` package](https://github.com/Nikaia-Foundation/Nikaia/tree/main/packages/yaml)
([#928](https://github.com/Nikaia-Foundation/Nikaia/issues/928),
[ADR-yaml-1](/docs/specification/adr/adr-yaml-1.html)). It reads YAML text
into a tree of nodes that point into the input rather than copy it, each with
the place in the text it came from, and it applies limits on depth and size by
default. Its parser is a `grammar`, part of the language itself
([ADR-296](/docs/specification/adr/adr-296.html)). YAML suits the question
well, because its rules depend on indentation and context, its input often
comes from outside the program, and there is a tuned parser to measure
against, `saphyr-parser`.

## Where it stands

The package passes all 402 cases of the yaml-test-suite, which it carries in
its own tests. Every index in its source is proved, and of its 101 arithmetic
operations 77 are proved and the other 24 are justified by a named bound.

The [benchmark](https://github.com/Nikaia-Foundation/Nikaia/blob/main/benches/yaml/README.md)
counts instructions with callgrind, with overflow checks on and off, and checks
that every program prints the same output byte for byte. On the large input,
10 MB of YAML, reading the events takes 2.20 times the instructions of
`saphyr-parser` with checks off, and building the tree takes 1.76 times those
of `saphyr::Yaml`. Reading into typed values through the facade takes 1.50
times those of `serde_yaml::from_str`, and the README traces that gap to the
parser. The same programs are shorter in Nikaia, 418 tokens for the events
against 616, and 388 for the tree against 554. The README gives the full
tables, the method and the causes still open.

We expect the package to reach `saphyr-parser`, and in the end to need fewer
instructions than it. That is a hypothesis these questions test. The present
target is at most 1.5 times `saphyr-parser` on the events
([#1548](https://github.com/Nikaia-Foundation/Nikaia/issues/1548)).

## The questions

### Is every bound check proved?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 70 % of the estimated whole, 20 % held back for what is not yet known"><span class="rq-known" style="width: 80%"><span class="rq-done" style="width: 88%"></span></span><span class="rq-unknown" style="width: 20%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 25%"></span></div></div>
<p class="rq-note">All 66 indexes in the package's source are proved, and 77 of its 101 arithmetic operations. The other 24 are justified by a named bound, such as a count of nodes or an offset into the input, and still carry their check. A package test prints these figures, so a change that loses a proof shows up there.</p>
</div>

The prover went through the package and proved every index without anything
written for it ([#1516](https://github.com/Nikaia-Foundation/Nikaia/issues/1516)).
The operations it does not prove yet are listed one by one in the package's
`bounds` test, each with the bound that keeps it safe. Several of them wait on
the prover following a value through a call's result into one of its fields,
and on counters that only rise
([#1557](https://github.com/Nikaia-Foundation/Nikaia/issues/1557)). The
question is whether all of them can be proved, so that the whole library runs
without a check on any index or operation, and how much of that the prover
does on its own. A proved check is left out only where a build asks for it
([ADR-306](/docs/specification/adr/adr-306.html)), so the figures above say
what can be left out, not what a default build leaves out.

### How close does it come to saphyr-parser?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 35 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 50%"></span></span><span class="rq-unknown" style="width: 30%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 5%"></span></div></div>
<p class="rq-note">The benchmark is built and the first causes are fixed. The events take between 1.64 and 3.04 times the instructions of saphyr-parser across the five inputs, with checks off. The gap has been narrowing as causes are fixed, which is all the evidence there is so far for the hypothesis.</p>
</div>

The benchmark compares the package's events with `saphyr-parser` and with
`unsafe-libyaml`, its tree with `saphyr::Yaml` and `serde_yaml::Value`, and its
typed reading with `serde_yaml::from_str`, on five inputs. With checks off the
events take 1.64 times the instructions of `saphyr-parser` on the small input,
2.20 on the large one, 2.53 on deeply nested input, 2.73 on long scalars and
3.04 over the test suite. On the large input that is 6703.0 million
instructions against 3040.4 million, and 7205.1 million against 3142.4 million
with checks on.

The README names the causes it found. Copying each event at every level of
nesting is fixed, which took the deep input from 5.4 to 2.5 times, and reading
properties twice is fixed, which took the large input from 2.5 to 2.2. Three
are open. A rule of the grammar clones the values it is given at each call,
about 4 % of the large input; a rule's values are to be lent like a function's
arguments ([ADR-grammar-1](/docs/specification/adr/adr-grammar-1.html) D6),
and the build is [#1560](https://github.com/Nikaia-Foundation/Nikaia/issues/1560).
Allocating and copying take about 15 % of the large input and wait on events
read straight from the parse
([#1518](https://github.com/Nikaia-Foundation/Nikaia/issues/1518)). Plain
scalars are read one character at a time, about 7 %, and wait on a fix in the
grammar engine. The question is how far the gap closes once these are built,
and whether the hypothesis holds that the package can reach `saphyr-parser`
and pass it.

### Does the compiler show where to optimise?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">A warning on a clone that copies nothing new is being built. Nothing has been measured yet.</p>
</div>

The causes above were found by reading callgrind's profile. A compiler that
knows which values a function keeps and which it only reads could point to
many of them while the code is written. The first such warning marks a
`.clone()` that copies nothing the program needs
([#1318](https://github.com/Nikaia-Foundation/Nikaia/issues/1318)). Once it is
built, it will be run on the package and its warnings set against the
callgrind profile, sorted into hot spots it names, hot spots it misses, and
warnings on code that hardly runs.

Some costs come from the lowering and not from anything the source writes,
such as the clone of a grammar rule's values at each call
([ADR-grammar-1](/docs/specification/adr/adr-grammar-1.html) D6,
[#1560](https://github.com/Nikaia-Foundation/Nikaia/issues/1560)). A warning on
the source cannot see these, so they form a class of their own, found in the
profile and fixed in the compiler. The question is how much of the work of
optimising a library the compiler can take on by naming where it goes.

### Can a grammar be the parser?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 60 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 80%"></span></span><span class="rq-unknown" style="width: 25%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 20%"></span></div></div>
<p class="rq-note">The package's parser is one grammar that builds the tree, and it passes all 402 cases of the yaml-test-suite. Its event program is 418 tokens against 616 for the one over saphyr-parser. Rules that take values, with bounds and guards, are built; lending those values instead of cloning them is decided and waits on its build.</p>
</div>

The package's parser is one `grammar` that reads the text and builds the
tree ([ADR-yaml-1](/docs/specification/adr/adr-yaml-1.html)
D3), and a grammar is part of the language
([ADR-296](/docs/specification/adr/adr-296.html)). YAML needs rules that take
values, such as the current indentation, and guards that decide which rule
applies, which [ADR-grammar-1](/docs/specification/adr/adr-grammar-1.html)
adds. Writing the package found defects in the grammar engine, which are filed
for the compiler, among them
[#1561](https://github.com/Nikaia-Foundation/Nikaia/issues/1561). The question
is whether a grammar written in the language can express a format as
demanding as YAML in full, and run as fast as a parser written by hand.

### Can it keep comments and layout?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 5 % of the estimated whole, 30 % held back for what is not yet known"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 7%"></span></span><span class="rq-unknown" style="width: 30%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence neither way yet"><span class="rq-conf-pos" style="width: 0%"></span></div></div>
<p class="rq-note">Every node already carries where in the text it came from. Comments, and the layout between nodes, are not yet in the tree.</p>
</div>

A configuration file that a program changes and writes back should keep the
comments, the order and the indentation its author gave it. The package's
plan has this as a later stage, which needs a tree that carries comments and
the place of every token
([ADR-yaml-1](/docs/specification/adr/adr-yaml-1.html) D10). Reading the text
and writing it back unchanged, comments and layout included, is
[#1518](https://github.com/Nikaia-Foundation/Nikaia/issues/1518), which also
lets the events be read straight from the parse. The question is whether one
tree can serve both fast reading and faithful editing, or whether the two pull
it apart.

### Does it feel familiar to the people who use it?

<div class="rq-bars">
<div class="rq-row"><span>Built</span><div class="rq-build" role="img" aria-label="Built: about 70 % of the estimated whole, 25 % held back for what is not yet known"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 93%"></span></span><span class="rq-unknown" style="width: 25%"></span></div></div>
<div class="rq-row"><span>Evidence</span><div class="rq-conf" role="img" aria-label="Evidence for a positive answer"><span class="rq-conf-pos" style="width: 15%"></span></div></div>
<p class="rq-note">The facade reads and writes with serde_yaml 0.9's functions and types. Of serde_yaml's own tests that the package runs, 96 pass and 2 do not yet. Typed reading takes 1.50 times the instructions of serde_yaml::from_str on the large input.</p>
</div>

A Rust program can use the package as a Rust library
([ADR-393](/docs/specification/adr/adr-393.html)), and the package offers it
the functions and types of `serde_yaml` 0.9, so that a program that uses
`serde_yaml` can switch to it and keep its calls. The facade reads with
`from_str`, `from_reader`, `Value` and `Deserializer`, and writes with
`to_string`, `to_writer` and `Serializer`
([#1515](https://github.com/Nikaia-Foundation/Nikaia/issues/1515)). It is
written in Rust over the package's own events and writer, and parts of it are
`serde_yaml`'s code, used under its licence. Its output matches `serde_yaml`'s
byte for byte on the writing tests. The question is whether a library written
in Nikaia can take the place of an established one without its users noticing
anything but the safety it brings, and what the facade costs in instructions
once the parser underneath is as fast as it can be.

## Ideas this rests on

| Idea | Recorded in |
| :--- | :--- |
| The official yaml package: one grammar builds the tree, nodes point into the input, limits on by default | [ADR-yaml-1](/docs/specification/adr/adr-yaml-1.html) |
| A grammar is part of the language | [ADR-296](/docs/specification/adr/adr-296.html) |
| A grammar's rules take values, with bounds and guards, and the values are lent rather than cloned | [ADR-grammar-1](/docs/specification/adr/adr-grammar-1.html) |
| A package becomes a Rust library, and hand-written Rust can sit over it | [ADR-393](/docs/specification/adr/adr-393.html) |
| A check is dropped only where a proof shows it cannot fail, and dropping is asked for | [ADR-306](/docs/specification/adr/adr-306.html) |
| Performance is counted in instructions against tuned Rust and not timed | [measuring](/docs/measuring.html) |
