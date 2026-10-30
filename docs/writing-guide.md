# Writing for readers outside the project

This guide is for the website, `README.md` and the guide (`guide/`), the text a
newcomer reads before deciding whether Nikaia is worth a closer look. The owner
set these rules on 2026-10-30. The specification (`docs/specification/`) is not
covered here; it keeps its own style, which states the rule and nothing else
(`docs/supervisor.md`, *Decision rounds*).

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
a program has to write (in tokens, `CLAUDE.md`, *Measuring performance*), the
measurement may be quoted. Not having to colour functions is a real advantage
and can be said plainly: the compiler finds where a function pauses, so nobody
has to rework the callers of a function that starts to wait.

## How claims are phrased

Absolute statements invite critics, who need only one counterexample to take
them apart. Words like "never", "impossible", "no failure is lost" or "always
safe" do not appear. Instead the text says what the compiler checks, and in the
same paragraph where that check ends. For example, the compiler refuses two
tasks that would write the same file at once, and it does not refuse where it
cannot tell what a call touches. Every task has a memory budget, and memory that
a Rust crate or a C library allocates for itself is not counted in it.

Phrasing is positive. Rather than "there is no `await`", the text says that a
program does not need `await`, because the compiler finds where a function
pauses. A missing feature is described by what takes its place.

Concepts are explained through what a programmer already knows. A task's error,
for instance, is thrown where the program waits for the task, and the compiler
points out that it has to be handled, just as it does for any call that can
fail.

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
