# Rules for an issue

An issue is the work order: a builder gets the issue, not a brief written
around it, and plans its own steps from it. So an issue holds everything an
agent cannot find out quickly by itself. These rules say what that is, so that
an agent can measure an issue against them after its run: was the issue bad
because its author broke a rule, or was the issue as the rules ask and the
rules missed something? Each rule has an id (`I1` ...) to cite in a report.

The rules describe how the good issues are written today (#1263, #1254). A
rule that turns out wrong or missing is changed here, with the run that showed
it, like the checks in `docs/supervisor.md`.

## Every issue

* **I1. Title names the symptom or the goal**, in words a reader knows
  without the issue: "a match on a field of a `mut` parameter moves it", not
  "NK0412 in check_refs" or a brief code.
* **I2. One status label**: `ready`, `decision`, `blocked` or `note`
  (`docs/README.md`), beside the type and the area label. `blocked` names the
  issue it waits on; `decision` holds a decision paper (options, examples, a
  recommendation).
* **I3. Plain language.** An error code stands beside its message, never
  alone; a function name beside what it does. No internal targets, dates for
  outside readers, or brief codes.
* **I4. Where it belongs**: `Part of #N` for the larger piece, `Found while
  #N` (or the commit) for where it came up, and the issues it depends on.

## A bug (something specified or plainly meant does not work)

* **I5. A probe that runs today**: the smallest program that shows it, with
  the command, the **expected** output and the **actual** output (the error
  with its code and message, or the `rustc` error on the emitted Rust). It was
  run on the binary of the day the issue was written; the issue says so.
* **I6. The governing decision**: the ADR and its decision (`ADR-282 D7`) or
  the section of Part I-III that says what is right. Without one, the issue
  says "no decision yet" and is a `decision` issue, not a bug
  (`CLAUDE.md`, *A missing form stops the work*).
* **I7. The cause, as far as known**: the file and function, and how sure
  (traced / suspected / unknown). With several shapes of the probe, which were
  traced to that cause.

## Work to build (bug or feature)

* **I8. Done when**: one sentence saying what is true when the issue is
  finished, checkable without the author.
* **I8a. Measured sizes** (2026-11-08, #1397, #1414): an item that promises to
  remove sites gives the count it measured per group, with one probe per group,
  and checks the group against the governing ADR's *does not decide* list; an
  estimate not measured says so.
* **I9. A numbered *To build* list.** Each item is one change a builder can
  commit on its own, and names its test: a `.nika` with `.stdout` / `.stderr`
  in `tests/compiler/` (or the package's tests), and the rows it must show.
  The order is the order to build in; an item resting on another says so.
* **I10. The rule that repeats**: when a fix widens or narrows a rule, the
  other functions that test the same condition are named (or the issue says
  none were found, and how it looked).
* **I11. Files and limits**: the files the work is expected to change, and
  what it must not change (another lane's files, a lowering it must keep, an
  output that must stay byte-identical). A move into Nikaia names the Rust it
  replaces and the `std` counterparts it needs (Part III 17); a missing one
  is its own issue, linked.
* **I12. What was ruled out**: approaches tried and dropped, and why, so the
  builder does not try them again. Empty when nothing was tried.

## Kept up to date

* **I13. The issue is the record**: a finding, a changed plan or a finished
  item goes into the issue (a comment, or the *To build* list ticked), not
  only into a report or a commit. An item found wrong is corrected in the
  issue before the next run.

## How a run judges its issue

After its run a builder answers, in its report and in its debrief, what the
issue lacked that would have saved it turns or a wrong turn:

* `Issue: complete` — nothing was missing.
* `Issue: broke I<n>: <what was missing>` — a rule above asks for it and the
  issue does not have it. The author fixes the issue; nothing changes here.
* `Issue: rules-gap: <what was missing>` — no rule above asks for it. The
  supervisor adds a rule or widens one, citing the run.
* `Issue: wrong I<n>: <what>` — the issue has it, but it is false (a probe
  that no longer fails, a wrong file, a wrong ADR).

Several answers are joined with `;`. The supervisor counts them per rule and
per author; a rule that is broken often is made easier to keep, a gap that
comes back twice is a rule.
