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
  #N` (or the commit) for where it came up, and the issues it depends on,
  each on a line `Needs #N` (or `After`, `Blocked by`, `Waits for`,
  `Depends on`): the driver does not queue an issue while one so named is
  open (`drive_depends_on`, #1819).
* **I4a. A claim of today's behaviour is read in the code** (#1706): a
  sentence saying what the compiler, a script or the driver does today names
  the function and file it was read in, or the run that shows it (a probe,
  I5; a `--bounds` row, I8a). A comment is not the code: #1694's item 3 said
  a partial run's issue is queued again, from a stale comment in
  `drive.nika`, and `drive_queue_add` skips a `#N` still in `batch.txt`
  (#1697). A claim of what the repository holds (a file tracked, ignored or
  generated) is read the same way, with `git ls-files` or `git check-ignore
  -v` (#1771: #1621 said a dependency's ledger is gitignored, and
  `examples/http/nikaia.contracts` and `nikaia.derived` are tracked).
  Code is named by its function, never by a line number alone (lines move
  with every commit), and a value it quotes (an exit code, a printed line)
  is read in that function (#1733: #1717 cited "line 228" and "exit 4" for
  the restart; the line was `drive_wake` and exit 4 its time-limit stop,
  the restart is `drive_restart`).
* **I4b. Code in an issue is written the way a program writes it**
  (#1730): a probe, a sketch of a new API, or an example use, in a bug or
  a feature. It is checked with `scripts/probe.nika --check` (stubs for what
  does not exist yet) before the issue is filed, and the result is quoted.
  Types are named the way a program names them (`Vec[Row]`,
  `HashMap[String, Value]`). The specification's own words (`Seq`, `Map`,
  ADR-105 D4) cannot be written in a program. A form the grammar refuses
  is named as one the issue needs (I6). A sketch written in it is not
  (#1730: #116's sketch wrote `db::Value.Text`, `println(v)` of an enum,
  which is NK1201, and `Seq`/`Map`; the builder respelled them all).
* **I4c. A failure of a script names the commit that ran, and is checked
  against `develop`** (#1801): an issue filed from a driver or script
  failure quotes the commit the run used (an ESCALATE line ends with
  `(drive.nika <sha8>)`, `drive_wake` in `scripts/drive.nika`; a driver
  keeps the script it started with until a merge brings a new one). Before
  filing, `git log <sha8>..origin/develop -- <script>` and `git log -S
  <function>` show whether a later commit already changed it; where one
  did, the issue names that commit and orders only what is still missing
  (often a test), or is not filed (#1801: #1784 was filed after its fix
  57de760b0 had landed, and its run built only a test).

## A bug (something specified or plainly meant does not work)

* **I5. A probe that runs today**: the smallest program that shows it, with
  the command, the **expected** output and the **actual** output (the error
  with its code and message, or the `rustc` error on the emitted Rust). It was
  run on the binary of the day the issue was written; the issue says so.
  A lowering or proof difference names the commit it was seen on (subject
  and sha, the binary built from it), and its probe is an uncached
  `lower-std` of a `git archive HEAD` copy, not a `probe.nika` snippet:
  without the commit nobody can bisect it once the tree moves on (#1793:
  #1763's `Found:` named neither).
  The expected output is written out (the text it prints, or the NK code and
  the gist of its message), never pointed at: "the specified result" or "what
  the spec says" leaves the builder to find it (#1737: #1550's issue).
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
  It says where it ran: the shared checkout with other agents at work, or a
  tree nobody else touched. A symptom of a rebuild, a cache, a changed
  lowered file or a refusal in a file the run did not edit is first checked
  against the inputs others touched during the run (`git status --short`
  before and after, the files' mtimes): an input changed mid-run makes the
  rebuild correct and the report no bug (#1762: #1759 had neither probe nor
  this context, and its cause was another agent's edit).
  The probe is copied whole from the file that ran, never retyped: its
  `use` lines, its error type, and the manifest table each key sits in
  (#1726: #1715's probe threw text, lacked `use std::time` and put
  `user-parallelism` under `[package]`, where it is ignored, not under
  `[build]`; it did not compile as written and cost three probe rounds).
  It is written out in the issue (or the `Found:` line), never pointed at
  by a path: a file in a run's scratchpad is gone, or in another session's
  scratchpad and unreadable, when the issue is built (#1824: #1751's Found
  named `/tmp/claude-…/scratchpad/…` and left out the expected result
  ADR-327 D9 gives; the supervisor's comment had to supply both).
  A probe of a missing method or `std` entry stops at that name, so the
  rest of it is never checked: it is run once more with a method that
  exists in its place (`sort_by_key` for `sort_by`), and it writes
  Nikaia's names, not Rust's (#1749: #1738's `sort_by` probe compared with
  `b.cmp(a)`, where Nikaia's method is `compare`, ADR-341 D2, so it could
  not give its expected output even once the entry was built).
  A site the bounds walk leaves checked in the toolchain is named by its
  `.nika` file and line, never by the lowered `.rs` it became, and its
  probe is `lower-std --bounds --bounds-trace FILE.nika:LINE` with the
  rows it printed (the goal and the facts asked), or a reduction that
  `check --bounds` shows on a scratch file; a probe that needs a whole
  `lower-std --bounds` to show the line costs every reader minutes and
  shows no cause (#1780: #1776's `Found:` named lines of `fixed.rs`).
  A `Found:` whose case is half in the run's reach and half a decision is
  split: the run fixes its part, the issue keeps the other, with its
  expected result (#1726).
  One defect per bug issue, each with its own probe: two claims are two
  issues, and a claim no probe shows is checked against the code first, since
  a behaviour a function does on purpose is no bug (#1677: a `Found:` merged
  two claims without a probe, and the second was the describer's rule).
  A cost (a read more per call) is no defect either: it is left out, or
  filed alone as a measuring issue with its count (`docs/measuring.md`)
  (#1752: #1554 put seven claims in one point, among them a closure
  capture `block.rs` keeps on purpose and a thread-local read per
  allocation).
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
  "Unclear whether by design" is neither: before filing, the author reads
  the refusal's code in the code table of `30-nikaia-tooling.md`, the form's section
  (`scripts/syntax.nika -- KEYWORD`) and the ADRs that name it, and either
  cites the one that settles it or says "no decision yet". Where the decision
  requires what the program got (the refusal or the action), the defect is
  what does not match it, often the message, and the issue orders that fix,
  not accepting the form (#1737: #1550 hedged, though Part II 10.1 and
  ADR-296 D33 require the action; the fix was the message).
  Where the governing decision specifies, on purpose, the behaviour the issue
  wants changed, the issue is no bug either: it is a `decision` issue that
  names the decision to amend and the options with their results, and the
  build waits for the amendment (#1771: #1621 was filed `ready` to fix what
  ADR-100 D3 specifies, "believed while the hashes match", and its first
  run stopped for the decision).
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
  A named function is one `scripts/where.nika -- NAME` finds today; a cause
  without one says `unknown`, never a guessed name (#1741: #1729 suspected
  `own_option_containers`, which does not exist). The file named beside it
  is the one `where.nika` reports for that function, copied from its
  output, never recalled (#1779: #1628 named `prover_walk.nika` for
  `outside_source`, which lives in `prover_calls.nika`).
  A traced cause is followed outward before filing: the other forms that
  reach the same function (a closure, a `std` method that takes one, the
  same call on a fixed-size `Array`) are run on the binary of the day,
  and each that fails is named as a shape of this defect, with its probe
  line; a `Found:` line does the same (#1761: #1754's `Found:` named only
  the `if`-arm refusal, and the same cause made `fn(a) { a }`, `sort_by`
  on typed lists and `Array::sort_by_key` rustc errors).
  A cause that says a function *lacks* a case is checked against every other
  place that handles that case (grep the case: `Div`, `quotient`), and for
  a bounds refusal against the site's goal and facts from `lower-std
  --bounds --bounds-trace FILE.nika:LINE`, which name the missing fact
  (#1792: #1786 suspected `lin_term` in `prove_terms.nika` lacked `/` and
  `%`; the walk's quotient rule existed, and the missing fact was the `u8`
  type's upper bound).
  A refusal at a call of a `std` function is checked against that
  function's entry in `crates/nikaia-std/std.contracts` (its `keeps`,
  `borrows` columns) before a walk is blamed, and every option and
  argument of the call is run as a shape, not only the one the probe used
  (#1830: #1797 blamed `keeps_expression` for `process::run`'s `env`;
  `keep_option` already walked options, the ledger said `keeps = ["args"]`,
  and `env_remove` and `stdin` failed the same way).
  A probe that mixes several conditions (outside input, a guard, a call) is
  also run with each one taken away, and the issue says which variant still
  fails: #1571 blamed the outside input, and the guard alone did not prove.

## Work to build (bug or feature)

* **I8. Done when**: one sentence saying what is true when the issue is
  finished, checkable without the author.
  The result it names is written out (the action the program then takes,
  or the NK code and the gist of the refusal), never pointed at: "gives the
  specified result" leaves the builder to choose between them (#1747:
  #1736's *Done when* left open whether the action or ADR-296 D25's refusal
  was meant; its probe broke I5 too, not copied from a file that ran).
  When the last item may end in filed issues instead of code, the issue
  says which code step carries `Fixes #N` (#1453).
  It never asks the run for the numbers of issues its own findings
  become ("each loss names its issue"): a builder files no issue, and the
  driver numbers its `Found:` lines only after the run. It asks for a
  `Found:` line per finding instead, which the driver files and links
  (#1843: #1611's and #1613's runs could not meet it).
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
  A *Done when* that promises the issue's own probe proved is held to
  the same: the probe is traced for every fact it needs, and a fact no
  item builds is named as a gap (a `decision`, I6) or the promise is
  narrowed to what the items reach (#1782: #1688 promised its probe
  proved by an "ascending" rule alone; the probe also needed each element
  of `limits` capped by `len(trail)`, which no rule gives).
  The trace names both sides of each site's goal: a sum or a product
  needs an upper bound of every operand, which `0 <= x` facts never give,
  and a difference a lower one; and each fact the issue proposes is
  checked against the form that would say it: a ledger `ensures` reads
  only the parameters and `result`, never `self` (`bm_ensured` in
  `bounds_matched.nika`), and a function parameter is bounded only by a
  precondition (ADR-269 D5). A fact no form can say is a gap (a
  `decision`, I6), not an item (#1829: #1811 expected `size + length` and
  `(buffer.len() - size) * 1000` proved from `0 <=` ensures and from
  `size <= buffer.len()`, which no ensures can state).
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
  An item settles each choice that changes what its result does, or says
  the builder chooses (#1733: #1717 left open whether a merge with no local
  commits fast-forwards, and a merge commit alone would have made the suite
  run).
  An item that removes or replaces code names what that code does today
  (I4a) and is checked against the decision it rests on (I6): code the
  decision itself requires is not ordered away (#1735: #1527's item 3
  removed http's spawn + join, which is how http answers 500 to a
  panicking handler, as ADR-326 D4 asks).
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
  A file an earlier issue is still to create (`After #N`) is named as that
  issue's output ("the harness #1612 builds"), never by a guessed path;
  when #N closes, the author puts the path it built in its place (#1843:
  #1613 named `benches/game/game.sh`, which #1612 built as `game.nika`).
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
  A group or item moved to another issue leaves this one in the same edit:
  its *To build* item is struck with `moved to #N`, and the *Done when*
  and its figures are narrowed to what is left, so no run is held to a
  promise the plan no longer makes (#1826: #1634's *Done when* still named
  the group-D counters the supervisor had moved to #1681, so its run could
  only end `partial`).
  A section that says where the work stands (a branch, uncommitted files)
  is checked against `git log` and `git branch -r` before each run (#1470:
  it named a stale branch and files already committed).
  So is each open *To build* item, before a brief is written for it:
  `git log --grep` on its topic. An item that another issue's work has already
  built is ticked, and the commit is named (#1730: #116's item 9 was
  already built when its run started).
  A bug's probe (I5) is run again on today's `develop` before its run is
  queued, and the search is by topic, not only by number, since a fix often
  lands under another issue: a probe that passes goes into the issue with
  that commit, and the rest of the issue (a pin test, the debt it names) is
  what the run builds (#1742: #1530's probe passed when its run started,
  fixed by 995d93f4 under #1507, the day #1530 was filed).

## How a run judges its issue

After its run a builder answers, in its report and in its debrief, what the
issue lacked that would have saved it turns or a wrong turn:

* `Issue: complete` — nothing was missing. Write that word, not `meets
  I1-I13`, and nothing after it: a note for the run's own issue is a
  `Comment: #N <text>` line, which the driver posts there; inside `Issue:`
  it is filed as an improvement issue (#1753: #1740's run wrote `meets
  I1-I13; comment: all three cases fixed ...`).
* `Issue: broke I<n>: <what was missing>` — a rule above asks for it and the
  issue does not have it. The author fixes the issue; nothing changes here.
* `Issue: rules-gap: <what was missing>` — no rule above asks for it. The
  supervisor adds a rule or widens one, citing the run.
* `Issue: wrong I<n>: <what>` — the issue has it, but it is false (a probe
  that no longer fails, a wrong file, a wrong ADR). `<n>` is the rule that
  asks for the false part; a false claim no rule covers is a `rules-gap`
  (#1706: a wrong claim of how the driver behaves was cited as I5, the probe).

`<n>` is looked up in this file before it is written, never recalled: I3
is plain language and I4 where it belongs; a probe that passes before any
change is I5 and so is one that does not parse or check as written (#1768:
#1673's run answered `wrong I3` for a probe writing `List[i64]`, NK1135), a decision called
unbuilt that is built is I6 ("built or text only"), a suspected function
that does not exist is I7, an item that is wrong or leaves open a choice
that changes its result without saying the builder chooses is I9 (#1735: #1527's run answered `wrong I3`
and `wrong I4` for an I5/I6 and an I9 point; #1708 the same slip; #1741:
#1729's run answered `wrong I3` for a probe that did not parse, I5, and a
suspected function that does not exist, I7; #1745: #1731's run answered
`broke I3` for an issue filed `ready` without its decision, I6; #1761:
#1754's run answered `broke I3` for shapes of the cause left unnamed, I7;
#1771: #1621's run answered `wrong I3` for a false claim of what git tracks,
I4a, and `rules-gap` for a fix of what an ADR specifies, I6; #1779:
#1628's run answered `broke I4` for a probe that was a fragment without
its `fn`, `main` and `use` lines, I5 (copied whole), and `wrong I5` for a
blamed cause and a wrong file, I7; #1780: #1776's run answered `broke
I3` for a `Found:` that named no cause, I7 (`unknown` at least), and
whose probe was a whole `lower-std --bounds`, I5; #1746:
#1533's run answered `rules-gap` for an item that left open what bounds the
wait before a body's first byte, `broke I9`).

A point the run has already closed as noise in its `Body:` (a mislabelled
rule number a driver-filed issue copied from an earlier report, a slip
with nothing to build) is not judged again in `Issue:`: the driver files
each `Issue:` answer as a new improvement issue, and the echo only files
the closed point once more (#1708: #1703's run closed its I3 label as a
citation slip and still answered `wrong I3`; #1802: #1791's run closed its
I5 label as the wrong rule, built the driver template fix itself, and still
answered `wrong I5` unmarked).

A point the run has already built in its own commit (it added the missing
rule to this file, or fixed the issue) is answered with `(built)` after its
kind, `rules-gap (built): <what was missing>`: it stays in the report for the
supervisor's count, and the driver does not file it (#1739: #1707's run
added its rule to I9 and its `rules-gap` was filed again as #1724; #1726's
run forgot the mark too and was filed as #1773, so `scripts/commit.nika` now
prints a `NOTE:` when a commit changes a rules file or a script under
`scripts/` (the driver's templates, #1802) and `Issue:` holds an unmarked
answer).

Several answers are joined with `;`. The supervisor counts them per rule and
per author; a rule that is broken often is made easier to keep, a gap that
comes back twice is a rule.
