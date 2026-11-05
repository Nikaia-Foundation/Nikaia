# Writing for readers outside the project

This guide is for the website, `README.md` and the guide (`guide/`), the text a
newcomer reads before deciding whether Nikaia is worth a closer look, and later
reads to learn the language. The owner set these rules on 2026-10-30 and added
the ones on other languages and on teaching on 2026-11-04. The specification
follows other measures, set out in the last section.

## What we lead with

Safety is the main argument, everywhere. In Nikaia it means more than memory
safety in the classical sense. It covers what happens when a task runs out of
memory, when a program forgets to wait for a task, when a task throws an error
while others are still running, and when two tasks touch the same file or value
at once. A page that talks about concurrency, errors or resources starts from
these questions and shows how the language answers them.

Simplicity is not something we can prove, so we do not argue it with claims.
Readers find it out for themselves, and the documentation supports that by
showing clear examples. Where something can be measured, such as how much less
a program has to write (in tokens, `docs/measuring.md`), the
measurement may be quoted. Not having to mark functions that wait is a real
advantage and can be said plainly, by what it does in Nikaia: the compiler
finds where a function pauses, so a function that starts to wait leaves its
callers as they are.

## How claims are phrased

Absolute statements invite critics, who need only one counterexample to take
them apart. Words like "never", "impossible", "no failure is lost" or "always
safe" do not appear. Instead the text says what the compiler checks, and in the
same paragraph where that check ends. For example, the compiler refuses two
tasks that would both write to standard output at once, and it does not refuse
where it cannot tell what a call touches. Every task has a memory budget, and memory that
a Rust crate or a C library allocates for itself is not counted in it.

Phrasing is positive. Rather than "there is no `await`" or "you do not need
`await`", the text says where the program waits: at a call that reads a file,
and for a value started with `async`, where the value is first used. A missing
feature is described by what takes its place.

Concepts are explained through what the reader already knows: ideas every
programmer has met, such as a file, a network call or a value that changes, and
what earlier pages of our own documentation have taught. A task's error, for
instance, is thrown where the program waits for the task, and the compiler
points out that it has to be handled, just as it does for any call that can
fail. What the reader knows from another language is not the starting point
(next section).

## Nikaia is explained on its own terms

The text says what Nikaia does, and does not compare it with other languages.
It does not say what one has to do in another language ("in JavaScript a
function that waits is marked `async`"), what Nikaia is like ("as in Go or
Rust") or what it is not like ("unlike Python"). A reader who knows the
compared language learns little from such a sentence, a reader who does not
learns nothing, and both read further before reaching what the page is about.
A page is written for someone who can program, without assuming which language
they come from, and it does not address readers of particular languages.

Three kinds of text name another language and stay. A measurement against
another language names it, because that is what was measured
(`docs/measuring.md`, the pages under `website/fast/`). Rust and C are named
where a program actually meets them: a Rust crate or a C library it calls, and
a message that still comes from the Rust compiler underneath. The decision
records and the manifesto argue for the language and may compare; the guide,
the website and `README.md` link to them rather than repeat the comparison.

The tone is that of Go's documentation: short sentences, the plain word, and
the thing itself from the first line. A section of the guide opens with what
the reader can write or do, not with background, history or a promise.

## How a page teaches

The guide teaches the way a good book on a language does, one idea after
another, each building on the ones before. A section shows a short program that
runs, says what it does, and then shows the mistake a reader is likely to make
with the same idea: the program, marked as one the compiler refuses, the
compiler's message in full, exactly as the current compiler prints it, and a
few sentences that read the message with the reader and arrive at the fix. A
page that introduces a check shows at least one such refusal, because the
message is where a reader meets the check in practice.

## How the text reads

The text is written in natural sentences and paragraphs. One pattern is avoided
in particular, because it reads as machine-written: a bold or leading term that
announces the topic, followed by an explanation ("**No failure is lost.** Every
failure reaches…", "Safety: …"). The same goes for colon lead-ins, bold
phrases inside running text, and lists of slogans.

When a paragraph deserves its own heading, it gets a real heading. The heading
describes what the section is about ("What happens when a task fails"), rather
than making a hard claim ("Failures are never lost"). Prose follows under it.
Lists are kept for things that are genuinely a list, such as the steps of an
installation.

## What is shown as working

A page never presents syntax or behaviour as runnable before the compiler
builds it. Where a page describes a design that is specified but not yet built,
it says so plainly and links the issue that builds it. Runnable examples are
checked against the current compiler before they are published.
`tests/page-examples` takes every `nika` example of `website/` and `guide/`
through `nikaia check` and `nikaia lower` and compares the result with its
`EXPECTED.txt`; CI runs it whenever an example changes. After changing one,
run `nikaia run > EXPECTED.txt` there and read the diff: a block that stopped
lowering is a defect in the page or in the compiler.

## The specification is written for a different reader

Parts I-III of `docs/specification/` are read in full by models that take them
into their context, so they are written for that reader and not for a
newcomer. Each rule is stated once, in one place, and other places point to it.
A word has one meaning across all three parts; when a keyword takes over a
word, older text that used the word in another sense is changed, headings
included. Nothing goes in that is not a rule: reasons, history, comparisons and
anything that argues for the language belong in the ADR that decided it
(`docs/supervisor.md`, *Decision rounds*). Text is kept short, because every
sentence costs every reader its tokens.

The opening sentence of a paragraph may state the rule in bold, with the
conditions and limits after it. Unlike on the website, this pattern helps here,
because a model reading the part finds the rule first. The claims of the
specification are rules the compiler enforces, so they are as absolute as the
rule is, and their limits are stated with them.

