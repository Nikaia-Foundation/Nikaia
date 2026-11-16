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
* **I2. One status label**: `ready`, `decision`, `blocked`, `wip` or `note`
  (`docs/README.md`), beside the type and the area label. `blocked` names the
  issue it waits on; `wip` is being built, by a lane or in the issues it names; `decision` holds a decision paper (options, examples, a
  recommendation).
* **I3. Plain language.** An error code stands beside its message, never
  alone; a function name beside what it does. No internal targets, dates for
  outside readers, or brief codes.
* **I4. Where it belongs**: `Part of #N` for the larger piece, `Found while
  #N` (or the commit) for where it came up, and the issues it depends on.
* **I4a. A claim of today's behaviour is read in the code** (#1706): a
  sentence saying what the compiler, a script or the driver does today names
  the function and file it was read in, or the run that shows it (a probe,
  I5; a `--bounds` row, I8a). A comment is not the code: #1694's item 3 said
  a partial run's issue is queued again, from a stale comment in
  `drive.nika`, and `drive_queue_add` skips a `#N` still in `batch.txt`
  (#1697).

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
  It is the whole command that fails, as run: which package is built and
  with which setting; where a library builds clean and only a program that
  uses it fails, the probe is that program (#1716: #1686's http failure
  showed only when hello-http was built at "yes", the http library alone
  built).
  The probe is copied whole from the file that ran, never retyped: its
  `use` lines, its error type, and the manifest table each key sits in
  (#1726: #1715's probe threw text, lacked `use std::time` and put
  `user-parallelism` under `[package]`, where it is ignored, not under
  `[build]`; it did not compile as written and cost three probe rounds).
  A `Found:` whose case is half in the run's reach and half a decision is
  split: the run fixes its part, the issue keeps the other, with its
  expected result (#1726).
  One defect per bug issue, each with its own probe: two claims are two
  issues, and a claim no probe shows is checked against the code first, since
  a behaviour a function does on purpose is no bug (#1677: a `Found:` merged
  two claims without a probe, and the second was the describer's rule).
  A builder's report gives each defect its own `Found:` line for this.
  Each shape the issue names as failing (a `for` and a `while` form) is
  run on the binary of the day; a shape that already works is named as
  working, not as part of the defect, and the cause (I7) is looked for in
  what the failing shape's path does that the working one's does not (#1695:
  #1654 named the `while` forms, proved on develop; only the `for` form
  failed, because `bounding_for` never asked the pair bounds
  `bounding_while` asks).
* **I6. The governing decision**: the ADR and its decision (`ADR-282 D7`) or
  the section of Part I-III that says what is right. Without one, the issue
  says "no decision yet" and is a `decision` issue, not a bug
  (`CLAUDE.md`, *A missing form stops the work*).
  A step that rests on a decision says whether it is built (the function
  that does it) or text only; text only is a dependency, named by its issue
  (I4) and built first (#1453).
  Code the decision or the issue quotes for the new form (a call, a
  signature) is spelled as today's grammar accepts it, checked with
  `scripts/probe.nika --check` on stubs; a spelling the language refuses is
  corrected in the ADR before the build is ordered, so the builder decides
  no syntax (#1687: ADR-289 D34 wrote `net::Tls.files(certificate:, key:)`,
  NK1230 and NK1101).
* **I7. The cause, as far as known**: the file and function, and how sure
  (traced / suspected / unknown). With several shapes of the probe, which were
  traced to that cause.
  A probe that mixes several conditions (outside input, a guard, a call) is
  also run with each one taken away, and the issue says which variant still
  fails: #1571 blamed the outside input, and the guard alone did not prove.

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
  An item that promises every listed site proved has traced each site
  (`--bounds-trace`) for all the facts it needs; a site that also needs a
  fact the walk lacks (`a.min(n)`) is listed under that gap (#1496).
  A site in a method of a type with field rules (ADR-306 D21, D28) has a
  probe reduced from the real code: the function and every `ref mut self`
  method of its type, with stub types, kept as the acceptance check; a
  synthetic probe passed while the real code stayed checked (#1470).
  Such an item also checks for each site that the rule can read the field:
  a `pub` field gets no found bound when its type can be seen from outside
  the package (ADR-306 D22). So the issue lists each site's field as private
  or `pub`, and a `pub` one counts only under an item that makes it private
  first (#1653: `SvRow`'s list fields are `pub`, so its `sv_*` sites could
  not move; #1629 made `SvSimplex`'s fields private for that reason).
  The probe's stub types keep each field's visibility as the real type
  declares it, so a `pub` site is never stood for by a private stub that
  proves (#1693: #1652's probe had private fields, its site `pub` ones).
  An item that asks for a rule over a list's elements ("every element of
  `a` is below `b.len()`") lists every write to both fields and checks the
  rule against each: a sentinel value written into an element (`-1` for
  "none") and a write that shrinks the bounding list (`truncate`, `pop`)
  each break it, and the item names the form that excludes them, or says
  none exists yet (a `decision`, I6). Its probe holds those writes, since
  without writes no field rule is found (#1719: #1668's step asked for an
  element rule on `place`, which holds `-1`, against `heap`, which shrinks).
  A probe that stands for sites keeps, at each site, the expression that
  builds the indexed value (a loop that pushes, an `if` expression, a
  `.clone()`, a field read), and the issue quotes those lines of one real
  site beside the probe, so the builder compares them before building; a
  probe of another form proves no site, and a *Measure* item on it shows
  no change (#1703: #1655's probe built `cols` by a loop over `ks`, the
  `sv_*` sites by `if ks.len() == 0 { [] } else { b.row.cols.clone() }`).
  Each group's probe is shown still checked on the binary of the day (its
  `--bounds` row in the issue); a probe that already proves counts nothing,
  and the issue names the real site instead (#1634: its probe was proved by
  D13 turn counting, and the real sites needed a field-entry fact).
  This holds for every claim of what the walk does today, not only a counted
  group: "today it stays checked" carries the probe's `--bounds` row from the
  binary of the day, and a figure from another prover (the solver) is named
  as such (#1643: the walk already proved `x % m`, the claim came from the
  solver, #1689).
* **I9. A numbered *To build* list.** Each item is one change a builder can
  commit on its own, and names its test: a `.nika` with `.stdout` / `.stderr`
  in `tests/compiler/tests/` (or the package's tests), and the rows it must show
  (#1470: an issue that said `tests/compiler/` sent the builder to the wrong place).
  Never a Rust test file or a `#[test]` (CLAUDE.md); a builder reads one an
  older issue still names as that program, and does not report it (#1671).
  The order is the order to build in; an item resting on another says so.
  An item is work a builder can do without GitHub: what to say on another
  issue is a report's `Comment: #N <text>`, which the driver posts, and an
  issue whose only point is such a request is closed as noise (#1698, #1707).
  A change to a default or a lowering level names where that level is set
  (`sysroot::tools_build` for `lower-std`, not `Build::default`), the ADRs that
  limit the crate the output lands in (ADR-218 for `nikaia-std`), and the `it`
  tests whose helpers lower with no flag (#1474, #1475).
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
  The limit says the command, the file and whether wall or user time, with
  the figure measured before (#1470: "1.2 times its time" without a
  baseline cost a run its measuring), and names any recomputation the
  pass already does per body that the change would multiply.
  Several steps in one pass get one limit on the whole run, or a per-step
  limit only where that step's own cost was measured or estimated (#1470: a
  1.2 times limit per probe could not hold the cost of D25's own checks).
  An issue that changes which facts the bounds walk asks with shows a
  `--bounds-trace` of its probe on the binary of the day (#1500: the
  trace showed the shared turn variable the rule had missed).
  The files and the *To build* items cover the whole scope *Done when*
  names: a *Done when* over "the compiler" or "every site" lists the sites
  found (count per file, the search that found them), and each kind of
  site the change touches (a set's `clear` beside its `insert`) is named
  (#1704: #1674 named 3 files, its *Done when* reached 73 sites in 21).
* **I12. What was ruled out**: approaches tried and dropped, and why, so the
  builder does not try them again. Empty when nothing was tried.

## Kept up to date

* **I13. The issue is the record**: a finding, a changed plan or a finished
  item goes into the issue (a comment, or the *To build* list ticked), not
  only into a report or a commit. An item found wrong is corrected in the
  issue before the next run.
  A section that says where the work stands (a branch, uncommitted files)
  is checked against `git log` and `git branch -r` before each run (#1470:
  it named a stale branch and files already committed).

## How a run judges its issue

After its run a builder answers, in its report and in its debrief, what the
issue lacked that would have saved it turns or a wrong turn:

* `Issue: complete` — nothing was missing.
* `Issue: broke I<n>: <what was missing>` — a rule above asks for it and the
  issue does not have it. The author fixes the issue; nothing changes here.
* `Issue: rules-gap: <what was missing>` — no rule above asks for it. The
  supervisor adds a rule or widens one, citing the run.
* `Issue: wrong I<n>: <what>` — the issue has it, but it is false (a probe
  that no longer fails, a wrong file, a wrong ADR). `<n>` is the rule that
  asks for the false part; a false claim no rule covers is a `rules-gap`
  (#1706: a wrong claim of how the driver behaves was cited as I5, the probe).

A point the run has already closed as noise in its `Body:` (a mislabelled
rule number a driver-filed issue copied from an earlier report, a slip
with nothing to build) is not judged again in `Issue:`: the driver files
each `Issue:` answer as a new improvement issue, and the echo only files
the closed point once more (#1708: #1703's run closed its I3 label as a
citation slip and still answered `wrong I3`).

Several answers are joined with `;`. The supervisor counts them per rule and
per author; a rule that is broken often is made easier to keep, a gap that
comes back twice is a rule.
