# Rules for an issue

An issue is the work order: a builder plans its steps from it, so it holds
what an agent cannot find out quickly. A run judges its issue by these rules
(last section); a rule found wrong or missing is changed here, citing the run.
Before filing, run `nikaia run scripts/issue-check.nika -- --body FILE` (or
`-- N`): the driver queues no issue it refuses (I4, I8, I9; #2003).

## Every issue
* **I1. Title names the symptom or the goal** in words a reader knows: "a
  match on a field of a `mut` parameter moves it", not "NK0412 in check_refs".
* **I2. One status label** (`docs/README.md`) beside type and area: `ready`,
  `blocked` (names the issue it waits on), `wip` (being built), `decision` (a
  decision paper: options, examples, a recommendation) or `note`.
* **I3. Plain language.** An error code beside its message, a function name
  beside what it does; no internal targets, dates or brief codes.
* **I4. Where it belongs**: `Part of #N`, `Found while #N` (or the commit), and
  each dependency on its own line `Needs #N` (or `After`, `Blocked by`, `Waits
  for`, `Depends on`); the driver does not queue an issue while one so named is
  open (`drive_depends_on`). A part another issue builds (a shared file) is its
  own issue named so, never a parent closing after its children; else the item
  says the builder makes it.
* **I4a. A claim of today's behaviour is read in the code**: it names the
  function and file (never a line number alone; a comment is not the code), or
  the run that shows it (I5, I8a); a value it quotes is read in that function.
  What the repository holds is read with `git ls-files` / `git check-ignore -v`.
* **I4b. Code is written the way a program writes it** (probe, API sketch,
  example), checked with `scripts/probe.nika --check` (stubs for what does not
  exist) before filing, result quoted: `Vec[Row]`, not the spec's `Seq`. A
  form the grammar refuses is one the issue needs (I6).
* **I4c. A script failure names the commit that ran and is checked against
  `develop`** (an ESCALATE line ends `(drive.nika <sha8>)`): where `git log
  <sha8>..origin/develop -- <script>` or `git log -S <function>` shows a later
  change, the issue names it and orders only what is missing, or is not filed.

## A bug (something specified or plainly meant does not work)
* **I5. A probe that runs today**: the smallest program that shows it, with the
  command, the **expected** and the **actual** output (code and message, or the
  `rustc` error), run on the binary of the day, which the issue says.
  * Expected output is written out, never "the specified result"; an exact
    figure only from a named source (a right reference run with its output, or
    the ADR), else the property that must hold ("never negative").
  * Copied whole from the file that ran (`use` lines, error type, manifest
    table) into the issue, never a scratchpad path; the whole command as run
    (package, setting; where only a program using a library fails, that one).
    It names what it needs to fail, so a test grown from it fails before the
    fix. Ownership probes use owned text made by a call (`fn make() -> String
    { let mut g = String(); g.push_str("hi"); return g }`), never a literal
    (the text-tier pass, ADR-282, makes it a `Copy` `&str`). A probe of a
    missing method is run again with an existing one in its place, in Nikaia's
    names (`compare`, not `cmp`).
  * A lowering or proof difference names the commit (subject, sha) and is
    probed by an uncached `lower-std` of a `git archive HEAD` copy; one between
    two runs is reduced to at most two files and says whether the function has
    callers in more than one file (ADR-391 D1). A toolchain site left checked
    is named by `.nika` file and line and probed with `lower-std --bounds
    --bounds-trace FILE.nika:LINE` (its rows) or `check --bounds` on a reduction.
  * It says where it ran (shared checkout or not); a rebuild, cache, lowered
    file or refusal in an unedited file is first checked against what others
    touched (`git status --short` before and after). One defect per issue,
    each with its probe.
  * Each shape named failing was run today, a working one is named working,
    and the cause is sought in what the failing path does differently. A claim
    no probe shows is checked in the code (on purpose is no bug). A cost is no
    defect (`docs/measuring.md`), nor is a census of sites by kind: only a group
    no gate or decision explains is filed, with one site's probe. A case half
    in a run's reach is split: the run fixes its part.
* **I6. The governing decision**: the ADR decision (`ADR-282 D7`) or Part I-III
  section that says what is right; without one, "no decision yet" and a
  `decision` issue (CLAUDE.md, *A missing form stops the work*).
  * Never "unclear whether by design": read the code's row in
    `30-nikaia-tooling.md`, the form's section (`scripts/syntax.nika -- KEYWORD`)
    and its ADRs, then cite one or say "no decision yet". Where the decision
    requires what the program got, the defect is the mismatch (often the
    message); behaviour it specifies on purpose is a `decision` issue naming
    the decision to amend and the options. A step resting on a decision says
    whether it is built (the function) or text only (a dependency, I4).
  * A feature, example or benchmark probes each form it rests on (input, output,
    `std` entry) before `ready`; one failing undecided is a `decision` issue,
    filed first, named by `Needs #N`. Code a decision quotes for a new form
    passes `probe.nika --check` on stubs, or the ADR is corrected first.
* **I7. The cause, as far as known**: file and function, marked `traced`,
  `suspected` or `unknown`, and which shapes were traced to it.
  * `traced` shows the trace (`--bounds-trace`, the lowered line, a stack; for a
    cost the instructions counted); read off a name, it is `suspected`, with the
    check that settles it. `nikaia run scripts/drive.nika -- --cause-check
    N|FILE` checks the marking before `ready`; a run checks a suspected or
    unmarked cause first, and one shown false is its `wrong I7`.
  * A named function is one `scripts/where.nika -- NAME` finds, its file copied
    from there; else `unknown`. A traced cause is followed outward: the other
    forms reaching it (a closure, a `std` method taking one, the `Array` twin)
    are run, each failing one named with a probe.
  * "A function lacks a case" is checked against every place handling the case
    (grep it) and, for a bounds refusal, the site's `--bounds-trace` facts; a
    refusal at a `std` call against its `crates/nikaia-std/std.contracts` entry
    (`keeps`, `borrows`), with every option and argument run as a shape. A probe
    mixing conditions (input, guard, call), or a guard of several conjuncts, is
    run with each taken away; the issue says which still fails.

## Work to build (bug or feature)
* **I8. Done when**: one sentence, checkable without the author, with the result
  written out (the action, or the NK code and gist); it says which code step
  carries `Fixes #N` and asks a `Found:` line per finding, never issue numbers.
* **I8a. Measured sizes**: an item removing sites gives the measured count per
  group, one probe each (for the toolchain a real site from `lower-std
  --bounds`, its row from the binary of the day), checked against the ADR's
  *does not decide* list; an estimate says so, a `Found:` count is measured
  first. A figure
  names the reason it counts (the walk's report line), each item the reason it
  moves, and the commit measured on (re-measured when a base issue landed). A
  site counts only where the decision's mechanism reaches its probe, listed by
  file and line, under the mechanism built last.
  * A promise of proof traces every fact each site (or the probe) needs, both
    sides of the goal; a fact no item builds or no form says (`ensures` reads
    only parameters and `result`; a parameter is bounded only by a precondition,
    ADR-269 D5) is a gap (I6). Every claim of what the walk does
    today carries its `--bounds` row; a solver figure is named as such.
  * A site with field rules (ADR-306 D21, D28) has a probe reduced from the real
    code (the function, every `ref mut self` method, stubs keeping field
    visibility and control forms, the expression building the indexed value),
    quoted beside one real site; each field listed private or `pub` (D22). A
    rule over a list's elements lists every write to both fields (sentinels,
    shrinking writes), names the form excluding them or says none exists (I6),
    and writes its induction step out (any allowed state, each `mut self`
    method, the rule after); a relation the prover cannot say is a gap.
* **I9. A numbered *To build* list** in build order; each item one change a
  builder commits alone, naming its test (a `.nika` with `.stdout` / `.stderr`
  in `tests/compiler/tests/` or the package's tests) and the rows it shows;
  never a Rust test (a builder reads an old one as that program). Each item
  settles every choice that changes its result, or says the builder chooses.
  Two sources offered as one ("the parsed parameter or the ledger's keeps")
  are read in the code first (I4a): where they differ on a case, the item
  names that case and picks one (#2010).
  * Two passes that each take a run are two issues (`After #N`). A fact made in
    one pass and read in another names how it reaches each reader's caller;
    recording what a pass reads names the function all reads go through (I4a),
    or counts the read sites; else `no decision yet`.
  * Each speed-up measured on its own is its own issue, so a run commits it and
    the driver pushes it within one run's time (#2012 held three).
  * A new refusal our own code must first pass is staged: the check behind a
    `warn` setting or warning code, the moves one item per area, the switch
    last; the issue names the setting or says `no decision yet`.
  * Removed code is named with what it does today (I4a) and checked against the
    decision (I6). No item needs GitHub (dispatch a workflow, set a secret
    or label): such a step is the supervisor's, a `Supervisor:` line after
    *Done when*, which does not wait for it; a builder meeting one in an
    older issue builds the rest, reports `Result: done` when that holds,
    names the step in `Comment: #N <text>` and judges `broke I9` (#2019). A
    changed default or lowering level names where it is set 
    (`sysroot::tools_build`), the crate's limiting ADRs (ADR-218) and the `it`
    tests lowering with no flag.
* **I10. The rule that repeats**: a fix widening or narrowing a rule names the
  other functions testing that condition, or how it looked for them.
* **I11. Files and limits**: the files expected to change and what must not
  (another lane's files, a lowering, a byte-identical output), covering all
  *Done when* reaches (sites per file, the search, each kind of site). A move
  into Nikaia names the Rust it replaces and the `std` counterparts (Part III
  17), a missing one its own issue. A file an open issue creates is named as
  its output, never a guessed path. Added Rust names what allows it (a
  CLAUDE.md exception, a dated decision, a move into Nikaia first), else `no
  decision yet` (I6).
  * Limits it runs into: caps (`most_facts()`), silently trimmed sets. A change
    to a pass every build runs names the time before (command, file, wall or
    user, commit) and per-body recomputation it multiplies; one limit over
    several steps unless a step's cost is known. A `lower-std` speed-up gives
    the split by pass (`NIKAIA_LOWER_TIMES=1`) and covers the largest. A change
    to the facts the bounds walk asks shows a `--bounds-trace` of its probe.
* **I12. What was ruled out**: approaches tried and dropped, and why.

## Kept up to date
* **I13. The issue is the record**: findings, changed plans and finished items
  go into the issue (a comment, the *To build* ticked); a wrong item is fixed
  before the next run, a moved one struck `moved to #N` with *Done when*
  narrowed. Before each run, a where-it-stands section is checked against `git
  log` / `git branch -r`, each open item by `git log --grep` on its topic (built
  ones ticked with the commit), and a bug's probe rerun on `develop` (a passing
  one is recorded with its fix, and the run builds the rest).

## How a run judges its issue
A builder answers in its report what the issue lacked, answers joined by `;`:
`Issue: complete` (the bare word; a note for the issue is a `Comment: #N
<text>` line), `Issue: broke I<n>: <what>` (a rule asks for it, the issue
lacks it), `Issue: rules-gap: <what>` (no rule asks for it; the supervisor
adds one) or `Issue: wrong I<n>: <what>` (the issue has it, but false; `<n>` is
the rule asking for that part, a false claim no rule covers is a `rules-gap`).

`<n>` is looked up here, never recalled (a probe that passes or does not check
is I5, a false claim of today's behaviour I4a). A point closed as noise in
`Body:` is not answered again; one the run's own commit built gets `(built)`
after its kind (`rules-gap (built): ...`), and the driver does not file it.
