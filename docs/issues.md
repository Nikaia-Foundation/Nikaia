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
  A proof that differs between two runs (`check` and `lower-std`, one file
  and a package) is reduced to a probe of at most two files, and says whether
  the function is called from more than one file (ADR-391 D1) (#1465).
  The probe names what it needs in order to fail (which writes keep a field
  rule alive), so a test grown from it still fails before the fix (#1483).
* **I6. The governing decision**: the ADR and its decision (`ADR-282 D7`) or
  the section of Part I-III that says what is right. Without one, the issue
  says "no decision yet" and is a `decision` issue, not a bug
  (`CLAUDE.md`, *A missing form stops the work*).
  A step that rests on a decision says whether it is built (the function
  that does it) or text only; text only is a dependency, named by its issue
  (I4) and built first (#1453).
* **I7. The cause, as far as known**: the file and function, and how sure
  (traced / suspected / unknown). With several shapes of the probe, which were
  traced to that cause.

## Work to build (bug or feature)

* **I8. Done when**: one sentence saying what is true when the issue is
  finished, checkable without the author.
  When the last item may end in filed issues instead of code, the issue
  says which code step carries `Fixes #N` (#1453).
* **I8a. Measured sizes** (2026-11-08, #1397, #1414): an item that promises to
  remove sites gives the count it measured per group, with one probe per group,
  and checks the group against the governing ADR's *does not decide* list; an
  estimate not measured says so. A *Done when* figure names the reason it
  counts (the walk's report line), and each item says which reason it moves;
  an item that moves none of the counted reasons does not promise the figure.
  A count taken from a builder's `Found:` is a hypothesis: the issue measures
  it, or makes the count its step 1 and stops when it is short (#1431).
  A figure of the toolchain is measured with `lower-std --bounds`, and each
  group's probe is a real site from that run, not only a test program
  (#1414).
  Each group's probe is run against the decision the issue builds on: an
  item counts a site only when that decision's mechanism reaches its probe
  (the D24 value range of a returned list does not reach a trait method with
  no body); the sites list goes into the issue by file and line (#1453).
  A site that needs several mechanisms is counted under the one built last,
  so that a group's count is what falls when its item is built (#1470).
  A site in a method of a type with field rules (ADR-306 D21, D28) has a
  probe reduced from the real code: the function and every `ref mut self`
  method of its type, with stub types, kept as the acceptance check; a
  synthetic probe passed while the real code stayed checked (#1470).
* **I9. A numbered *To build* list.** Each item is one change a builder can
  commit on its own, and names its test: a `.nika` with `.stdout` / `.stderr`
  in `tests/compiler/tests/` (or the package's tests), and the rows it must show
  (#1470: an issue that said `tests/compiler/` sent the builder to the wrong place).
  The order is the order to build in; an item resting on another says so.
* **I10. The rule that repeats**: when a fix widens or narrows a rule, the
  other functions that test the same condition are named (or the issue says
  none were found, and how it looked).
* **I11. Files and limits**: the files the work is expected to change, and
  what it must not change (another lane's files, a lowering it must keep, an
  output that must stay byte-identical). A move into Nikaia names the Rust it
  replaces and the `std` counterparts it needs (Part III 17); a missing one
  is its own issue, linked.
  It names the limits the work will run into: a cap the code keeps (the
  walk's 48 facts, `most_facts()`), a set that is silently trimmed (#1470).
  A change to a pass every build runs (the bounds walk runs in each
  `lower-std`) names the time before it, on one named file, as a limit
  (#1470: a rule that made the walk 4.5 times slower stopped the run).
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
