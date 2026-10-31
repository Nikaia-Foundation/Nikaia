# The architect

How work is organised when the owner starts a session. Token rules:
[`tokensaverstrategy.md`](tokensaverstrategy.md); commit rules: `CLAUDE.md`;
the main session's duties, cycle and hand-over: [`supervisor.md`](supervisor.md).
History is in git and the issues, not here: a rule keeps one incident tag
(brief, date), and `git log -S '<tag>'` finds the story.

## Roles

| Role | Who | Job |
| :--- | :--- | :--- |
| Owner | the person | Sets goals, stages and priorities; decides what is theirs |
| Main session | the session the owner started, all tools | Talks to the owner; runs `scripts/drive.nika`; starts the architect and builders; raises versions; watches CI and cost |
| Architect | `.claude/agents/architect.md`, Opus, `effort: medium` (`high` only when the owner asks, for one task) | Decides what is built next and how; writes ADR and specification text, briefs and issues; improves the process |
| Builder | `.claude/agents/builder.md`, no GitHub | Builds one or two steps, commits locally after cheap checks, reports, ends; the driver runs the suite and pushes |
| QA | `.claude/agents/qa.md`, Sonnet, no GitHub | Started by the driver on a red suite; reads the unpushed briefs and reports, fixes forward, commits locally |

The architect cannot start agents: it coordinates by briefs, and the main
session (or `drive.nika`) runs them. A builder that hit an error does not go
on: its report comes to the architect, whose next order is the repair (the
compiler defect first, `CLAUDE.md`'s *Fix, do not work around*), then the
blocked step.

**Pushing from a checkout that is behind** (a dirty shared tree, so no
rebase): push a merge, `T=$(git merge-tree --write-tree HEAD origin/develop)`,
then `git commit-tree $T -p HEAD -p origin/develop` and push that sha. Never
rebuild your files on `origin/develop`'s tree with a scratch index: a file
another session changed there is overwritten (architect 2026-10-31: the y21
probe rule, restored by f96bb09a). Chain such steps with `&&`, never `;`.

## Language decisions (owner, 2026-10-31)

Language principles (owner, 2026-10-31), the measure for every language
decision: complexity goes into the compiler, not to the user; whoever uses
something pays for it, nobody else; consistency and harmony with what the
language already says; and a look at how other languages solve it, to do it
better.

* **Obvious: the architect decides.** A gap a builder or a move met (a `std`
  entry, a method, a form, a conversion) whose answer the principles settle -
  the form other parts of the language already have, a `std` counterpart of a
  Rust call, an extension nobody pays for who does not use it - is approved by
  the architect: ADR or specification text, the build filed `ready`, the
  blocked work re-queued behind it. The ADR names this rule.
* **Not obvious: the owner decides.** Where the principles pull apart (a cost
  for every user, a break with what is written, a choice between two
  consistent designs), or the owner's goals or stages are touched: a
  `decision` issue with a decision paper (`docs/supervisor.md`, *Decision
  rounds*). In doubt, it is not obvious.

## Reported defects are filed (owner, 2026-10-31)

Every report with `Result: partial|blocked` and a `Found:` defect with a
probe is filed by the architect at its next run: a GitHub issue in brief form
(`ready`, the probe, *To build*) and a bug brief
`docs/briefs/bugs/b<N>-<issue>-<topic>.md` with a `Blocks:` line, appended to
`docs/briefs/bugs/batch.txt`, issue labelled `wip`. An open issue that already
holds the defect gets the new probe as a comment instead. A lane architect whose task
forbids other lanes' briefs (`docs/briefs/prover/architect-task.md`) files
the issue `ready` without `wip` and writes no bug brief: the bug lane's
architect claims it (prover run after p6, 2026-10-31).

## Driver, suite and QA (owner, 2026-10-31)

Builders commit locally and push nothing. After each builder or architect
run the driver runs the complete suite (`scripts/gate.nika -- --all`, one at
a time per checkout, lock `target/suite.lock`, in its own detached worktree
at the tested sha, `/home/user/nikaia-suite`); green, it pushes the tested
commit; red, it writes `qa-input.txt` and starts QA
(`docs/briefs/qa-task.md`), at most two rounds, then `ESCALATE`. CI stays the
final gate for what other sessions push meanwhile.

## Lanes

The score in [`priorities.md`](priorities.md) decides; every brief names its
score terms in 2-3 lines. Each lane has its own `DIR` with `batch.txt`, its
own driver run and its own architect task (`DIR/architect-task.md`):

* **Main lane, `docs/briefs/`: self-hosting moves only** (owner, 2026-10-31).
  A batch holds at least two moves; the one exception is a fix-forward for a
  red `develop`. A compiler defect a move finds is not fixed in this lane:
  the architect files it (`ready`, brief form), writes its brief into
  `docs/briefs/bugs/` with a `Blocks: <move brief>` line, adds it to
  `docs/briefs/bugs/batch.txt` after the briefs listed there, and sets `wip`
  on the issue. The move waits: it re-enters the main batch once the fix is on
  `develop`, and the lane goes on with other moves meanwhile. Nothing is
  written around the defect (`CLAUDE.md`, *Fix, do not work around*).
* **Bug lane, `docs/briefs/bugs/`**: `ready` defects (kind 3 of `CLAUDE.md`,
  *A missing form stops the work*). A brief with a `Blocks:` line goes before
  the lane's own picks. Its own `NK` block, no version raise (`--no-bump`).
* **Other lanes** claim their issues with `wip` (the `yaml` lane,
  `docs/briefs/yaml/`, #928). An issue labelled `wip` is another lane's.

## Briefs and reports

A brief is one file `DIR/<topic>.md`, tracked (never a scratchpad: a fresh
session has none). `scripts/agent.nika` starts the agent with `Read your brief
at <path>`. It opens with this frontmatter (defaults `builder`, `sonnet`, 120,
20):

    ---
    agent: builder
    model: sonnet
    max_turns: 100
    budget_usd: 20
    ---

A line `Facts: <heading>, <heading>` (or `facts:` in the frontmatter) names
the `## ` / `### ` sections of `docs/builder-facts.md` the builder needs;
`agent.nika` puts their text into its prompt under *Facts for this brief*, so
it does not read the file whole. A heading ending in a parenthetical is named
without it (`Moving Rust into Nikaia`); an unknown one stops the run.

**Issues are written in brief form** (owner, 2026-10-25): frontmatter, facts,
*Locations*, *To build* steps with tests, rules, report form, so a builder can
start on one without the architect. Builders have no GitHub access: a brief
copies the issue's problem, probe and *To build* steps.

**A brief names the place** (owner, 2026-10-31):

* **Locations**: the file and function of each place to change, in Rust and in
  `tools/*.nika`, and the test file of a touched NK code, resolved with
  `nikaia run scripts/where.nika -- NAME...` (definitions, callers, tests) and
  `nikaia run scripts/where.nika -- --outline FILE` (a file's functions), never
  `grep -r`. A bug brief names the **suspected location** of the fault (file
  and function) that the architect found while tracing the probe, and says how
  sure it is.
* **`Facts: <heading>, ...`**: the sections of `docs/builder-facts.md` the
  builder needs, by their exact `##` heading; `agent.nika` inlines them. A new
  trap goes into builder-facts, not into each brief.

**Before a brief is written, the architect checks** (each tag is the run that
paid for the rule):

* every probe and test row it prescribes runs as expected on today's binary,
  and agrees with Part I and the ADRs of every form it uses (b905, b995, y5);
  with several shapes it says which were traced to the named cause (y8);
* every grammar form and tree shape it plans is probed, across rules (y2); a
  prover claim is probed at the limit it claims, an open probe narrowed to one
  form (y7a, y7b);
* a step resting on another brief's fix opens with a pre-flight probe of it;
  a draft (`wip/` branch, patch) is applied and run through `nikaia check` and
  one test first (y2c);
* a step that builds new code over borrowed data is probed in its planned
  shape, not only in the shape of the last fix: the walk's recursion, its
  result type with view fields, the `match` payloads and `for` elements it
  reads, the by-value arguments beside them (y21 run 2: three checker defects
  a ten-line probe would have shown before the batch);
* a probe of a view-checker defect is built like the case it came from: the
  struct's field must really be a view there (built from a `trim()` inside a
  function, not from a literal in `main`, which makes the field owned), or the
  probe passes and the brief is wrong (y21 run 3, #1085). Before re-queueing a
  draft that stopped on one, rewrite the refused part in scratch and `check`
  and test the rest, so all its defects come out in one batch;
* a fix that widens a rule names every function that repeats that rule
  (the same condition tested elsewhere, e.g. `views_the_call_s` beside the
  emitter's #703 branch) and allows its file, or drops the row (b11);
* the expected lowering agrees with the ADR that governs it, or the brief says
  *any lowering rustc accepts* (b933);
* an emitter fix that needs what the checker knows names the fact that carries
  it (`check_facts.nika`, by `node_key`) or the field to add, and allows those
  files; a missing or extra `&` at a call is the checker's
  (`check_references.nika`, `the_compiler_writes_the_reference`) (b1007, b1023);
* a move behind a host probes each new host form with an impl in Nikaia
  (k3a2), and names each host method's full signature and how per-unit code
  reaches Rust-held data (m3a);
* every Rust `std` call a move moves has a Nikaia `std` counterpart (Part III
  17); a missing one is its own `std` brief, decided by the architect when
  obvious (*Language decisions*), else a `decision` issue with a decision
  paper. A brief never prescribes a way around a missing form unless
  it cites the ADR that refuses it, and a `blocked` report on a gap is
  answered with that issue (p3a; `CLAUDE.md`);
* a move that reshapes a struct names every file that reads its fields, with a
  count, the new shape, and any Nikaia twin that exists (p1);
* a route a brief calls possible ("one way needs no backend change") is
  probed by the architect in the failing case it must carry (a Cut, a
  template rule the macro inlines), the generated Rust read, before the brief
  offers it (j6 run 1: the backend's `item N` frame vanished on a Cut);
* a fix-forward lists every commit of the first red run's push
  (`git log <last green>..<first red>`), cheap suspects first
  (b-fixforward-sweep).

**Brief rules:**

* **No optional step.** Every step is what `done` means; split into two briefs
  instead (b867).
* **Time box** (owner, 2026-10-31). A builder that has found no cause after
  about 40 turns, or goes in circles, stops and reports `Result: partial` with
  what it ruled out, the probe, and where it suspects the fault
  (`.claude/agents/builder.md`). The architect then narrows the probe or
  re-traces; it does not re-queue the same brief. A long brief tells the
  builder to stop about 15 turns before `max_turns`, commit what is finished
  and report `partial` (y4).
* **The fixed block** `docs/briefs/common.part` (rules, disk, report form,
  `Attribution:`) ends every brief, also a fix-forward, with `<full brief
  path>` filled in: without the report path `batch.nika` never sees it done.

**A move brief** (owner, 2026-10-28) hands over its draft: one `Cut:
crates/<crate>/src/<file>.rs:A-B` line per range (400-600 lines a brief), one
`Convert: --profile check|check-free|emit [--prefix X_]` line (`--prefix`
whenever the target file prefixes its names, P4b) and one `Shape:` line.
`agent.nika` converts before the agent starts and names the draft. The brief
names the callees' locations, the syntax gaps the draft hits, the tests
between steps, and the callers of what moves including
`crates/nikaia/tests/it/` (P4b).

**A move is faithful unless a named reason forces a redesign** (owner,
2026-10-29):

* `Shape: faithful` (default): the same functions, bodies and data; Rust that
  stays Rust is reached through one-line adapters with unchanged signatures.
* `Shape: schematic <pattern>...`: a fixed rule the converter applies to every
  occurrence. A pattern rs2nika does not know goes under *Needs* with a
  three-line Rust probe and the Nikaia it must become; the move enters
  `batch.txt` only after the converter commit that teaches it.
* `Shape: redesign <reason>`: only for (1) data held by a crate or type that
  stays Rust and no adapter reaches, or (2) a shape `std` or the lowering
  cannot say yet, with the issue. Not taste, names, "cleaner". No `Convert:`
  line (so no 60 % rule), the report writes `Rs2nika: none (redesign)`, and
  the brief states the full target shape. A redesigned part under a third of
  the brief splits off into its own brief.

Converter patterns learned: methods to prefixed free functions with a `mut`
parameter (a53f456); `mem::take` of a field to `drain()` (fab482f); `env::var`
to `host.env_var` (1b08236). Wanted, in order: (1) `--prefix` on every free
function, type and call; (2) a method of a struct that stays Rust to a free
function over the fields it reads; (3) a named Rust call to a host method, in
general; (4) Rust error and path calls to `std` (`refuse`/`.context` to
`throw`, `ancestors()` to a `parent()` loop, `PathBuf` to `fs::Path`); (5) in
a loop, `let Some(x) = e else { continue }` to `let x = e ?? continue`.

### The report

The builder writes exactly `<full brief path>.report.md`, these fields only:

    Result: done | noop | partial | blocked   (noop: nothing to change; batch.nika counts it done)
    Files: <paths of the commit, space-separated>   (for scripts/commit.nika)
    Commit: <subject line>
    Body: <one paragraph for the commit message>   (optional)
    Fixes: #N[, #M] | Part: #N   (optional)
    Pushed: <sha> | rejected | none   (drive.nika rewrites `rejected` when it pushed the commit itself)
    Tests: <test filters run> - pass | <failing names>
    Gate: green | red <tests>   (#852; batch.nika cuts on red, #862)
    Deviation: none | <one sentence>   (bare `none`, no note after it: batch.nika cuts on anything else)
    Found: none | <one sentence, issue number if filed>
    Needs: none | <NK code, architect decision, owner decision, help from main session>
    Changelog: none | <one sentence for the version heading>
    Owner: none | <one question for the owner, never a role>   (batch.nika: `OWNER <brief>: <text>`; cycle.nika writes the first to DIR/QUESTION, a second while it exists prints `OWNER second: ...`)
    Cache: none | <large entries newly created, MB>
    Disk freed: <GB>
    Measure: <builds=.. clippy_with_tests=.. ...>   (never under Found)

Every brief ends with an `Attribution:` block (the commit's trailer lines);
`scripts/commit.nika` copies it into the message, with everything after it:
nothing follows the `Attribution:` block, and a paragraph added on a re-run
goes above `## Rules` (y2b 2026-10-31: an "Earlier attempt" section after it
would have landed in the commit message). The builder's final answer
is one line: `done <sha>, report <path>`, `BLOCKED: <reason>, report <path>`
or `PUSH REJECTED <sha>, report <path>`, plus `NEEDS: <short>` when `Needs`
is not `none`.

The architect reads the reports in its next task. It files or updates issues
from `Found`, comments on or reopens an issue a `Deviation` leaves open, lists
what the change decided for the supervisor's version heading, and picks the
next batch.

## Batches

Two or three briefs per batch (owner, 2026-10-25), in order. The driver starts
the next brief when one says `done`; `Found`, a `Deviation` other than none,
`partial`, `blocked` or `Gate: red` cuts the batch, and the briefs left come
back to the architect with the reports. While a batch runs, the architect
gives a new order in its answer and the main session writes `batch.txt`.
The brief is the work order: the architect's answer names the brief paths
in order, one start line each (agent, model, why), and what it needs from the
main session or the owner.

## Stages

The owner sets the stage. 1 Serial: one builder at a time; a defect found on
the way is repaired first. 2 Two: a second builder on files the first does not
touch. 3 More: by the owner, with stage 2's figures. Now (owner, 2026-10-26,
widened by `docs/bootstrap-week.md` 3): stage 1 within each lane, the lanes
side by side; throughput comes from scripts; the main session is woken only on
an escalation.

## Self-optimisation

Every cycle the architect looks at the reports for waste (polling, repeated
builds, a missed rule, a chafing tool, a missing script), changes the rule or
the tool at the source, and notes the figure in [`tokenspeed.md`](tokenspeed.md):
tokens and turns per finished step, steps per day; counted, not timed.

## State

*Current state only, rewritten (not appended) by the architect when it ends:
open lanes, what blocks what, figures, decisions in force that no ADR or issue
holds. What happened is in `git log` and the issues.*
**2026-10-31 (after 0d8deb1f).** Self-hosting: about 82 % of the toolchain
(`scripts/self_hosting.py`). Plan: `docs/bootstrap-week.md` (deadline
2026-11-04), rows and `Cut:` lines in `docs/selfhosting-plan.md`.

* **CI**: last known red (clippy 1.99) fixed forward by 91510ca0. Main
  session: `rustup update stable` at a quiet moment (this machine's `stable`
  is 1.97; 1.99.1 is installed beside it).
* **Main lane** (`docs/briefs/batch.txt`): C1a re-run (`c1a-ledger-items.md`,
  #1080 and #1082 built; old report `.run1.report.md`), K3b re-run
  (`k3b-comptime-bundle.md`, #1077 and #1078 built; cherry-pick 9a1678a8,
  never merge the wip branch: it sits on pre-rebase commits). GR and s1082
  done (ADR-384 landed). To check when next near `trust.rs`: `join` makes a
  package that reads no untrusted source all-fast even where keys come through
  a trait barrier (ADR-010 D9 point 5); file it if a probe shows a keyed
  position written fast. **P5c blocked** on #1088 (bug lane b20); move on
  branch `wip/p5c-project-drive` cc40334f, the brief's *Re-run* says how to
  land it (check it cherry-picks, as K3b's did not merge). **S1** is free
  (b17 built; an uncommitted `sysroot.rs` of another agent may still sit in
  the main checkout); `String.get(range)` is refused by design (NK1214).
  P5e waits on the O rows (#923).
* **Next main batch**: P5c re-run (after b20), C1b (`lookup`, `candidates`,
  the `stored` fold), S1. Owed, each its own brief: R0 (#921),
  `scripts/plan_rows.nika`, the driver writing the failing job's clippy output
  into `architect-input.txt`, fold `ComptimeRan` into `CheckWall`. The
  driver's `drive_push` rebase stopped once on a conflict in a lowered file
  (`lowered/check_facts.rs`, 2026-10-31 21:30) and was resolved by hand: if it
  recurs, the driver should re-lower on conflict in `lowered/*.rs`.
* **Bug lane** (`docs/briefs/bugs/batch.txt`): b17 #1077, b19 #1080 done;
  b18 #1078 done (46824f2b, local, `wip` stays until promote closes it), its
  Founds filed as #1097 (flat_map over a `Seq` result, `ready,wip`, decided on
  the issue: a `Seq`/`Vec`/`Array` result binds `U`) and #1098 (named fn and
  `filter_map` `if`/`null` over `iter()` of copy elements, `ready`). Batch:
  b20 #1088 (Blocks P5c), b21 #1097, b13 #1057, b22 #1099 (hole-less f-string receiver keeps `{{`), b23 #1100 (walk over a list literal of open numbers not copied); both s1082 Founds. Queued after, briefs written
  and `wip`: b14 #947, b16 #1065. #1060 stays open for step 2 (not
  reproduced). `ready` defects no lane holds, next: #1098, #1081 (`let a = call().field` borrows the temporary), #1075, #1074,
  #1083, #1086, #1066, #1038, #958, #1014, #927, #1056, #1058. Unfiled:
  qa-task's note (dev-profile `cargo test -p nikaia` hits the recursion limit
  in winnow-grammar `rt.rs:902`); a two-view `fn` field written with `String`
  instead of `ref String` is NK1106; `String?` == `String` refused (NK1102;
  GR keeps `sysroot_holds` for it): decide whether by design when next met.
  A suite gate-error that does not reproduce: re-run the suite before
  suspecting a commit. A bare `RED` line in `architect-input.txt` (no sha, no
  "failing jobs" sentence) is not the driver's: check the newest develop runs
  (`gh api .../actions/runs?branch=develop`) before treating it as red; at
  da12a9ca it was stale, CI green.
* **yaml lane**: #928, #929, #604 (`wip`).
* **Owed to the supervisor**: version raise / Changelog lines for s922b
  (`fs::Metadata::modified()`, `links()`, `file_id()`, `time::since_epoch()`,
  #922, ADR-319, ADR-368), s920, s922, b9, b10, b11, b6-b8, b1046; ADR-384
  lands with s1082's Part III text. **Waiting on the owner**: #1047, #1042.
* **In force, in no other file**: `drive.nika`'s `drive_push` runs `git pull
  --rebase --autostash` in the shared checkout. Branch
  `wip/bounding-candidates-2026-10-19` is parked.
