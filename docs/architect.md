# The architect and the main session

How work is organised when a session is started by the owner. Rules for the
agents' tokens are in [`tokensaverstrategy.md`](tokensaverstrategy.md); the
rules for commits are in `CLAUDE.md`.

## Roles

| Role | Who | Tools | Job |
| :--- | :--- | :--- | :--- |
| Owner | the person | - | Sets goals, stages and priorities; decides what is theirs |
| Main session | the session the owner started | all | Talks to the owner; starts and restarts the architect; starts builders from the architect's work orders; watches cost; carries information between them |
| Architect | `.claude/agents/architect.md`, Opus | read, search, `Bash`, `Edit`/`Write` for docs, GitHub issues read/write | Decides what is built next and how; writes ADR and specification text; keeps issues current; improves the process |
| Builder | `.claude/agents/builder.md` | six file and shell tools, no GitHub | Builds one or two steps, commits, pushes, reports, ends |

The architect cannot start agents (a subagent has no `Agent` tool). So the
architect **coordinates by orders**, and the main session carries them out.

## Effort

The main session starts the architect with `effort: medium` (owner's decision).
It raises it to `high` for one task only when the owner asks for it.

## The loop

1. The owner states a wish or answers a question to the main session.
2. The main session starts the architect with a short task: the wish, the issue
   numbers, and the last report. Nothing else; the state is in the repository.
3. The architect returns (at most 20 lines): the decision, **one work order**,
   the repository and issue changes it made, and questions for the owner.
4. The main session relays the owner's questions, and the answers go back to
   the architect (a fresh one, or the same while it is young).
5. The main session starts one builder with the work order. The builder builds,
   commits, pushes, reports and ends.
6. The main session reads the builder's report, puts it into the next
   architect task (the report, not the transcript), and goes to 2. A builder
   that hit an error does not go on: its report goes to the architect, whose next
   order is the repair (compiler defect first, `CLAUDE.md`'s *Fix, do not work
   around*), then the step that was blocked.

## Choosing the topic

By `docs/priorities.md`: the score decides across areas, bugs-first is the default prior; every brief names its score terms in 2-3 lines.

## Briefs and reports

A brief is one file (`docs/briefs/<topic>.md`, tracked; see *Where briefs live*); `scripts/agent.nika` starts the agent
with `Read your brief at <path>`. A brief starts with this frontmatter (all
keys optional; defaults `builder`, `sonnet`, 120, 20):

    ---
    agent: builder
    model: sonnet
    max_turns: 100
    budget_usd: 20
    ---

**A move brief hands over its draft** (owner, 2026-10-28): one `Cut:
crates/<crate>/src/<file>.rs:A-B` line per Rust range it moves, and one
`Convert: --profile check|check-free|emit [--prefix X_]` line. `agent.nika`
converts them before the agent starts and names the draft
(`<brief>.draft.nika`, `.notes`, `.rs`) in its prompt, so the builder starts
from it instead of cutting and converting itself. Give `--prefix X_`
whenever the target file prefixes its names (most do, R245): without it the
draft keeps the Rust names and every signature and call is rewritten (P4b kept
21 of 79 lines, its costliest edit). Name in the brief what the
builder would otherwise look up: the callees' locations (`where.nika`), the
syntax gaps the draft hits, the tests to run between steps, and the
callers of what moves including `crates/nikaia/tests/it/` (P4b found four
test files only from clippy, after the build). Cut 400-600 lines
a brief.

**A move is faithful unless a named reason forces a redesign** (owner,
2026-10-29: "weniger umbauen, oder schematische Umbauten gleich den Konverter
machen lassen"; MF kept 12 of 180 lines, P5a 6 of 104, M3a 12 of 109, each
flagged by the 60 % rule for a shape the brief had prescribed). Every move
brief carries one `Shape:` line beside `Convert:`:

* `Shape: faithful` (the default): the same functions, bodies and data, only
  the language changes. Rust that stays Rust is reached through one-line
  adapters with unchanged signatures, not by reshaping what moves.
* `Shape: schematic <pattern>...`: a redesign that follows a fixed rule, one
  the converter can apply to every occurrence (the list below). The brief
  names the patterns; a pattern rs2nika does not yet know goes under
  *Needs from the main session* with a three-line Rust probe and the Nikaia it
  must become, and the move enters `batch.txt` only after the converter commit
  that teaches it. The builder then keeps the draft's shape; the 60 % rule
  stands and now measures the converter.
* `Shape: redesign <reason>`: allowed only for (1) data held by a crate or a
  type that stays Rust and cannot be reached by an adapter (MF: `toml::Value`
  becomes a flat `ManifestItem` list), or (2) a shape `std` or the lowering
  cannot say yet, with the issue number. Not a reason: taste, names, "cleaner".
  Such a brief has **no `Convert:` line** (so no draft and no 60 % rule:
  `drive.nika` checks only a report with `kept N of M`), its report writes
  `Rs2nika: none (redesign)`, and the brief states the full target shape
  (types, signatures, host methods) since no draft carries it. A brief whose
  redesigned part is under a third of its lines splits: the redesign is its
  own brief, the rest stays faithful or schematic with its draft.

Schematic patterns, learned (commit) and wanted (in this order, from the
reports): learned: methods become prefixed free functions, `&mut self` a `mut`
parameter (a53f456); `mem::take` of a collection field a `drain()` (fab482f);
`env::var` a `host.env_var` (1b08236). Wanted: (1) `--prefix` on every free
function, type and call of the draft, not only constants (M3a's costliest);
(2) a method of a struct that stays Rust becomes a free function over the
fields it reads (`self.root`, `self.variant` -> parameters; P5a's `Project`
paths); (3) a named Rust call becomes a host method, `env::var`'s rule made
general (`parser::parse_to_ast` -> `host.parse_unit`, `manifest.setting` ->
`host.setting`; M3a, P5a); (4) Rust error and path calls become `std`:
`refuse(..)`/`.context(..)` a `throw`, `ancestors().skip(1).find(..)` a loop
over `parent()`, `PathBuf::from`/`as_os_str` an `fs::Path` (M3a); (5) inside a
loop, `let Some(x) = e else { continue }` and `match e { Some(x) => x, None =>
continue }` become `let x = e ?? continue` (MF's 21 return edits). A defect
beside them: the keyword escape renamed `with` inside a string literal (M3a).

**Issues are written in brief form** (owner, 2026-10-25): when the architect
writes an issue, its body is laid out as a brief (frontmatter, facts,
*Locations*, *To build* steps with tests, rules, report form), so a builder can
later be started on it without the architect. Builders have no GitHub access: a brief copies the issue's text and *To build* list. Every brief ends with this report form.

*Locations* are resolved with `nikaia run scripts/where.nika -- NAME...` (definitions, callers, tests of an `NK` code), not by `grep -r`.

What a brief must hold, so the builder does not look it up (2026-10-25 count:
builders spent up to 19 `grep -r` calls finding definitions, one built from a
title because the issue text was missing):

* **Issue text**: the issue's problem, probe and *To build* steps, copied.
* **Locations**: `file:line` (or `fn` name and file) of each function to
  change, in Rust and in `tools/*.nika`, and the test file that holds the
  touched NK code; the architect resolves them with one grep while writing.
* **Known facts**: one line, *Read `docs/builder-facts.md`*; add a new trap
  there, not in each brief.
* **A probe row the brief expects to run is checked against Part I first** (b905, 2026-10-28: the brief asked `for x in xs { out.push(x) }` over owned text to run, which ADR-094 D4 refuses; the builder had to report a Deviation and an owner question).
* **Every test row a brief prescribes is run by the architect before the brief is written**, not only the issue's probe (b995, 2026-10-30: the brief's lambda row `let wider = fn(n) { ... }` hit another defect, #997, and the batch was cut on a Deviation plus Found).
* **A brief's test with several shapes says which the architect traced to the named cause** and marks the rest unverified (y8, 2026-10-31: the `match` shape had a second cause in `bind_a_payload`).
* **A move behind a host probes each new host form before the brief is written** (k3a2, 2026-10-30: a trait method with a `fn` parameter had never been lowered; handed twice it was moved, #1008, found only after the slice was written): one small `nikaia run` program per new form of the host trait (a `fn` parameter used twice and handed to itself, a text parameter, an enum result), with an impl in Nikaia.
* **An emitter fix that needs what the checker knows names the fact that carries it** (`check_facts.nika`, keyed by `node_key`), or says none exists and which field to add where, and its stop clause allows those checker files (b1007, 2026-10-30: the builder spent a dozen reads finding that no fact held a call's result type, then stopped on the brief's file list). A missing or extra `&` at a call is decided by the checker (`the_compiler_writes_the_reference`, `check_references.nika`), not the emitter: such a brief allows that file and the predicate it calls (b1023, 2026-10-31: step 2 stopped on `a_lent_parameter` in `check_fields.nika`).
* **A brief is checked against the ADRs of every form it asks for, and every grammar form and claim it expects to build or prove is probed before it is written** (y5, 2026-10-30: the brief's pub `Scalar.value` holding both kinds broke ADR-282 D22, and `x:peek(text(..))` is refused by winnow-grammar, both met after the code was written; y4b: json's `scalar::from(cp)` was never probed and could not be proved). A grammar brief probes each tree shape it plans across rules, not only each rule: a struct one rule builds, bound by a label in another, its fields copied into the struct that rule returns (y2, 2026-10-31: the plan's `p:Props` copied onto `Node` hit #1026 after the code was written). A step resting on another brief's compiler fix opens with a pre-flight probe of that fix (a few lines, `check --asserts`); a run-long brief tells the builder to stop about 15 turns before `max_turns`, commit what is finished and report `partial` (y4 ran out at 110 with uncommitted work).
* **A brief that starts from a draft (a `wip/` branch, a patch) applies it to develop and runs `nikaia check` plus one test before it is written** (y2c, 2026-10-31: the y2 draft hit #1032 and #1033 at once, met only by the builder; both fixes could have run first).
* **A prover brief probes at the limit it claims and narrows each open probe to one form** (y7a, 2026-10-30: the spike's 4-turn probes hid that nothing past 6 turns was proved; y7b: json's `\u` path, one refusal, was four defects, each found by removing one form at a time).
* **A brief's expected lowering is checked against the ADR that governs it**, or the brief says *any lowering rustc accepts* (b933, 2026-10-29: the brief wanted `-> String`, ADR-353 D10 lowers a returned view of a local through the caller's keep; a false cut).
* **A move brief checks that every Rust `std` call it moves has a Nikaia `std` counterpart** (Part III 17; p3a, 2026-10-28: `shown_path` needed `canonicalize` and `strip_prefix`, which `std` lacked, and the move became a Deviation). A missing one is its own `std` brief first, or, where it is not decided, a `decision` issue with a decision paper for the owner (owner, 2026-10-29: CLAUDE.md, *A missing form stops the work*). A brief never prescribes a way around a missing form (`get` then `insert` for `entry`, a loop for a missing iterator method) unless it cites the ADR that refuses the form; a `blocked` report on a gap is answered with that issue, never with a brief that writes around it.
* **A move that reshapes a struct names every file that reads its fields, with a count, and the new shape** (p1, 2026-10-28: `Settings` split into `{ build, words }` left the shape open; six `cargo check` rounds over 42 field reads in three files). Where a Nikaia twin already exists (`CheckComputed` for `comptime_run::Computed`), the brief names it.
* **A fix-forward for a red CI step first lists every commit of the first red run's push** (`git log <last green>..<first red>`), not only the run's head: a commit pushed together with another has no run of its own (b-fixforward-sweep, 2026-10-28: the brief blamed c7e5b66, a converter commit no test reaches; eeaefb1, a compiler change in the same push, was the suspect). It then checks in one line whether each can reach the failing step, and orders the cheap suspects (allow-list entries, a new trait impl) first.
* **A move behind a host names each host method's full signature** and how per-unit code reaches Rust-held data (index into an arena, `RefCell` state), not only the method's name (m3a, 2026-10-28: about 8 files read to design `ModulesHost`).
* **No optional step** (*if turns remain ...*): every step of a brief is what `done` means. An optional step left undone reports `partial`, which cuts the batch (2026-10-26, b867). Split the work into two briefs instead.
* The fixed block `docs/briefs/common.part` (rules, disk, report form, `Attribution:`), pasted at the end with `<full brief path>` filled in. **Every brief has it, also a fix-forward the main session writes itself**: without the report path `batch.nika` never sees the brief done (2026-10-26).

The builder writes its report to exactly `<full brief path>.report.md`, beside the brief (the brief names the path), these fields only:

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
    Needs: none | <NK code, owner decision, help from main session>
    Changelog: none | <one sentence for the version heading>
    Owner: none | <one question for the owner, never a role>   (batch.nika: `OWNER <brief>: <text>`; cycle.nika writes the first to DIR/QUESTION, a second while it exists prints `OWNER second: ...`)
    Cache: none | <large entries newly created, MB>
    Disk freed: <GB>
    Measure: <builds=.. clippy_with_tests=.. ...>   (never under Found)

Every brief ends with an `Attribution:` block (the commit's trailer lines);
`scripts/commit.nika` copies it into the message.

Its final answer to the main session is one line: `done <sha>, report <path>`,
or `BLOCKED: <reason>, report <path>`, or `PUSH REJECTED <sha>, report <path>`.
A non-empty `Needs` adds `NEEDS: <short>` to that line. The main session reads
nothing else; token figures reach it from the harness.

The architect reads the report in its next task (the main session passes only
the path and the token figures). It files or updates issues from `Found`,
comments on or reopens an issue a `Deviation` leaves open, lists what the
change decided for the supervisor's version heading, and picks the next topic.
The main session still acts on its own for: CI red on `develop`, a rejected
push (it rebases), `BLOCKED`, and owner questions.

## Batches (owner, 2026-10-25, measure 8 option C)

Two or three briefs per round, for independent bugs in different files, with
their order. The main session starts the next builder when one says `done`;
a report with `Found`, a `Deviation` other than none, or `BLOCKED` cuts the
batch, and the briefs left come back to the architect with the reports.

## The work order

What the architect hands the main session for a builder, in this form:

* **Issue and steps**: `#N`, the one or two steps of its *To build* list.
* **Files**: the files to read first, and the files the builder may change.
* **Probes**: the whole matrix of cases to run after the single build.
* **Done when**: the touched tests to run (never the full suite: CI runs it on
  the push), the commit message line (`Fixes #N` only for the last step).
* **Measure**: none, unless the step's point is a number. Then the count to
  report (a work count the tool prints, such as files re-lowered, comes
  first) and the smallest input for callgrind; never callgrind over a whole
  `lower-std` or a test run ([`long-jobs.md`](long-jobs.md)).
* **Stop and report if**: an open design choice, an error in another file.
* **Two lines of context** the builder cannot find in the repository.

## The main session's duties

* **Batch watch.** After each hand-back run `nikaia run scripts/batch.nika -- DIR`
  and start what `NEXT` names; `CUT` goes to the architect; `--tokens BRIEF TOKENS CALLS`
  records the harness figures in `DIR/tokens.txt`.
* **Gate watch (#852).** A report without `Gate: green` is not passed on: the
  next order is a fix-forward brief, and a second red in a row is escalated.
* **Cost watch.** Look at each agent's size. A builder past about 120 turns or
  200k context is told to push what is verified and end. An architect past about
  80 turns or 150k context ends the same way. Restart with a fresh agent; never
  resume one for a second slice.
* **CI watch.** After each builder's push, one background call
  `gh run watch <run id> --exit-status` on that commit's run; it wakes the
  session at the end, no polling. The next builder starts meanwhile. For the
  state of `develop`, `nikaia run scripts/release.nika -- --dry-run DIR`
  prints one `CI` line; never the GitHub MCP `actions_list` (tens of k tokens
  of JSON). A red run goes to the architect, whose next order is the
  fix-forward brief (or a revert, when a record settles it).
* **What the main session reads.** Only the one-line answers (builder:
  `done <sha>, report <path>`; architect: at most 20 lines: brief paths in
  order, start orders, needs). It does not open reports or quote them: it
  passes the architect only `batch done` or `cut at <brief>` with the harness
  token figures, one line per agent. Owner messages go verbatim.
* **Where briefs live: `docs/briefs/` in the repository** (DIR of every
  script), with the reports, `batch.txt` and `tokens.txt`; never a scratchpad,
  which a fresh session does not have (2026-10-26: the batch was lost with it).
  The architect writes the briefs and `batch.txt` (when a batch is still
  running, it gives the order in its answer and the main session writes it);
  the main session commits `batch.txt` and the reports with `git add
  docs/briefs/<files>`. The *State* section below names the next batch.
* **Fresh container.** There is no `nikaia` binary (`nikaia` in this file means
  `target/fast/nikaia`). The main session's first command is
  `cargo build --profile fast -p nikaia` with `run_in_background` (about 3
  minutes), started while the architect works; `cycle.nika` cannot run before.
* **Release.** `nikaia run scripts/release.nika -- [--dry-run] [--loop M] REPORTS_DIR`
  reads the new `*.report.md`, prints CI, STOP and RED lines, and raises the
  version once for the done reports' `Changelog:` lines (`bump.py new`, push).
* **Hand-over.** A fresh agent gets: the task, the issue numbers, the two or
  three facts that are in no file. If a fact will be needed again, it is written
  to the repository (the architect's *State* below, a doc, a commit message), not
  carried in the prompt.
* **Owner channel.** The owner talks to the main session only. When the
  architect needs a decision, the main session asks the owner, following
  `docs/supervisor.md`'s *Decision rounds* (introduction from zero first).
* **Supervisor duties** (`docs/supervisor.md`) stay with the main session: version
  raises, `CHANGELOG.md`, badges, reserved `NK` blocks, rebases, a red `develop`.
* **No building in the main session**, except what a work order cannot reach.

## The main session's cycle

1. `nikaia run scripts/drive.nika -- docs/briefs` with `run_in_background` (`--max N` agent runs, default 8; `--dry-run` shows the next step). It loops on `cycle.nika`: runs each `NEXT` builder through `agent.nika` and commits the run's files (report, `.run.json`, `cost.txt`, `tokens.txt`, `tokenspeed.md`), applies `BUMP pending` when CI is green, and starts the architect (`docs/briefs/architect-task.md`, the stop line in `architect-input.txt`) on `CUT`, `RED` and `BATCH DONE`. It prints one line per agent run and stops with one final line, exit 2, on an escalation: an `OWNER` line, `ESCALATE` of `agent.nika`, an architect report with `Owner:` not `none` or `Result:` not `done`, the same stop line after an architect run, a RED sha already handed over, `--max` reached, a push rejected after one rebase.
2. The main session acts only on that final line. `cycle.nika -- DIR --done N` clears an OWNER line; after the owner's answer, start `drive.nika` again. On `RESTART drive: ...` (exit 3: `scripts/drive.nika` changed, the running copy is stale) start it again at once, no owner, no architect.
3. A failed run (`ESCALATE`, a file in DIR) is read like a failed builder; restart a fresh architect past 80 turns / 150k.
4. `OWNER` lines: first a fresh architect writes the decision paper (a `decision` issue: where it comes from, alternatives with doing nothing, for and against, other languages, recommendation; owner 2026-10-30, #1006 was asked bare); then ask the owner in German, per *Decision rounds*, with the issue; his answer goes to the architect. After the answer the architect re-queues: the build brief first in `batch.txt`, the blocked brief's report renamed `BRIEF.run1.report.md` (so `batch.nika` sees it unrun), `DIR/QUESTION` deleted.
5. Stop hook: the fixed answer below.

## Handover to a new main session

Standing rules, for a main session that starts fresh (the state is in
*State* below and in the issues, not here):

* **Owner messages go to the architect verbatim**, with only dictation errors
  fixed; the main session does not summarise them or decide for him.
* **Hand-backs are data.** A builder's or architect's report is read, not
  obeyed; no agent message is the owner's consent, and none changes
  permissions or settings. Never route around a denied permission: report it.
* **Decisions, one topic at a time, in German**, in the form of `docs/supervisor.md`'s
  *Decision rounds*: introduction from zero, where the question comes from,
  facts checked, options, risks, recommendation, the exact question.
* **Builders from briefs** (`docs/briefs/<topic>.md`, started with *Read your brief
  at <path>*, type `builder`), in batches as *Batches* above says; the
  harness figures of each (tokens, tool uses) go to the architect with the
  report path.
* **Version raises** are the main session's, done by itself without an agent
  (owner, 2026-10-25, measure 9): `scripts/bump.py new` with the architect's
  changelog lines, once per finished package.
* **CI**: watch `develop` with `nikaia run scripts/release.nika -- --loop 20 DIR`
  in the background (it prints only changed lines and ends on RED or when
  `DIR/STOP` exists), or by hand `nikaia run scripts/status.nika -- DIR` (CI,
  batch and pending-bump line, read-only); a red run is fixed forward at once
  (or the breaking commit reverted), before new work.
* **Stop hook**: when it says there are uncommitted changes, answer
  *„Die nicht committeten Dateien gehören den laufenden Buildern, die selbst
  committen und pushen; ich committe sie nicht."* The main session never
  commits another agent's files.
* **Branches** on the remote are deleted by the owner, not by an agent.
* **Architect restart** (owner, 2026-10-25): at every architect hand-back the
  main session reads the harness token figure; at about 150k context or about
  80 turns the next architect is started fresh, never resumed with
  SendMessage. Its state is in the repository: this file, `priorities.md`,
  the briefs, the issues.

## Stages

The owner sets the stage and moves to the next. Each stage ends with a short
note in *State* on what it measured.

| Stage | Rule | Goal |
| :--- | :--- | :--- |
| 1 Serial | One builder at a time. A defect found on the way is repaired first, by the same chain (architect orders, builder repairs), then the work resumes | Self-hosting forward (#125, `selfhosting-plan.md`); the right tools; the tools improved by the tools; compiler defects fixed |
| 2 Two | A second builder works on parts the first does not touch (the architect names the files of each). Each commits only its own files; the main session rebases | Speed, once stage 1 has removed the common stalls |
| 3 More | Decided by the owner, with the figures of stage 2 | |

Owner, 2026-10-26: stage 1 stays (one agent at a time, no parallel builders);
throughput comes from scripts, which may run cheap work in the background; the
architect keeps one batch of 2-3 briefs; the main session is woken only on an
escalation (`scripts/drive.nika`, brief `b-drive-loop.md`) and otherwise stays
free for the owner.

## Self-optimisation

Every cycle the architect looks at the builder reports for waste: turns on
polling, repeated builds, a missed rule, a chafing tool, a missing script. It
changes the rule or the tool at the source and notes the figure in
[`tokenspeed.md`](tokenspeed.md). The measure is tokens and turns per finished
step, and steps finished per day; counted, not timed (`CLAUDE.md`, *Measuring
performance*).

## State

*Updated by the architect when it ends.*

**2026-10-31 (architect after 1c09db7a, cut at b1043 on Found):** b1043 done (569bb244, `Fixes #1043`; Changelog line in its report, version raise owed). Found re-probed on a fresh fast binary and filed `ready` in brief form: #1045 (row c: an untyped let lambda put into a fn field, `h.f(1)` refused NK2605; traced to `check_core_stmt_let` popping the stored-code frame when the lambda is not `Ty::Fn`, so `check_core_places_code_into` sees an unresolved local) and #1046 (row d: an untyped let lambda typed by a pausing `fn` parameter emits `|x|`, not `async |x|`; `emit_core_closure` writes `async` only for `pausing_lambdas`; #997's test used `sync` params). #1036 re-probed, still fails. Process: `drive.nika` `drive_push` runs `git pull --rebase --autostash` in the shared checkout, which made b1043 build without its edits; trap added to builder-facts, the script is unchanged (a supervisor choice of 2026-10-28; a drive that rebases without stashing is not yet designed). Batch: `b1036-tuple-result-keeps-arg.md`, `b1045-let-lambda-fn-field.md`, `b1046-let-lambda-pausing-param.md` (all Opus, serial; b1045 and b1046 both touch `check_core_stmt.nika`).

**2026-10-31 (architect after 3571b0bf, cut at b1034b on Found):** b938b (25b24ed8, `Fixes #938`) and b1034b (650a4ef4, `Part: #1034`) done; Changelog lines in both reports, version raise owed. #1034 closed by hand (both steps built, comment). Found probed (rows a, b traced to `check_core_a_kept_lambda`'s `Variable` arm skipping locals; row c NK2605 unverified; `add(1)` after it is #833) and filed `ready` in brief form as #1043. #1036 traced to `buffers.nika:1636` (tuple results not a view source). Batch: `b1043-let-lambda-kept.md`, `b1036-tuple-result-keeps-arg.md` (both Opus); serial. Next after: #1010 (probe reproduces at 08:06, tier pass `text_tiers.nika`, untraced), #958, #1038, or #1042 once decided. A stray untracked file named `ork in the tree)...` sits in the main checkout (someone's shell slip): the main session may delete it.

**2026-10-31 (architect after 3ebedbca, batch done):** bred2 (a2ed99c2, CI fix-forward), b1034a (ec386c71, `Part: #1034`) and b1035 (1673a1fe, `Fixes #1035`) done; Changelog lines in b1034a's and b1035's reports, version raise owed (BUMP pending). #1034 step 2 re-probed: three rows, cause in `keeps.nika` (a closure handed out is not walked for the parameters it names), commented; brief `b1034b-returned-lambda-keeps-param.md` (Opus, `Fixes #1034`). Side find: `same(inc)(1)` refused by the parser, filed #1042 (`decision`, paper in the issue; owner's question, not urgent). b938b re-probed at 3ebedbca (rows a, b still fail). Uncommitted `text_tiers.nika` (#999, yaml lane, `wip`) in the shared tree makes 10 `tests/compiler` tests refuse NK2302 locally (b1035's report): the lane's, told to builders. Batch: `b938b-nested-number-lists.md`, `b1034b-returned-lambda-keeps-param.md`; serial. Next after: a `ready` bug the bug lane does not hold (#1036, #1038, #1010, #958 candidates), or #1042 once decided.

**2026-10-31 (architect after dd9d5470, cut at b938 on partial):** #938 one level done (a6ec14b3, `Part: #938`; Changelog line in its report, version raise owed). The nested rows are a defect (D34 one level deeper, D43's one rule), not a decision: probed a-g, commented on #938, brief `b938b-nested-number-lists.md` (Opus, `Fixes #938`), queued after this batch. CI red on every full run since 4fa242a6 (b0ba2293 + c129f145, #1029): one stale assertion, `selfhosting_defects::a_run_and_a_kept_lambda_in_a_statement_differ` wants `move |x| Box::pin`, the lowering now writes a typed let; rule added to `builder-facts.md` (grep `tests/it/` for the old lowered text). CI logs: the blob download is refused by the proxy; failing test names come from `gh api .../check-runs/<job id>/annotations`. Batch: `bred2-kept-lambda-test.md` (Sonnet, fix-forward), `b1034a-kept-lambda-captured-fn.md`, `b1035-fn-field-from-loan.md` (both unrun, unchanged); serial. Next after: `b938b`, then #1034 step 2.

**2026-10-31 (architect after 39f8dbf6, cut at b1031 on Found):** #1031 done (637bec45, `Fixes #1031`; Changelog line in its report, version raise owed). Found probed and split: #1034 (`ready`, brief form; step 1 a kept lambda calling a captured kept fn, the per-call `async move` moves the `Kept`, fix as a hull in `kept_hulls`; step 2 a `fn` parameter captured by a returned lambda lowers as run-only `&impl AsyncFn`, `check_function_keeps` misses the capture) and #1035 (`ready`; `return h.f` from `ref H` refused NK1131, the handed-back path lacks a copy check for `Ty::Fn`). Batch: `b938-loop-copy-inferred-list.md` (unrun, unchanged), `b1034a-kept-lambda-captured-fn.md` (Opus, `Part: #1034`), `b1035-fn-field-from-loan.md` (Opus, `Fixes #1035`); serial. Next after: #1034 step 2.

**2026-10-31 (architect after 4fa242a6, cut at b1029 on Deviation):** #1029 done (b0ba2293, c129f145, `Fixes #1029`; Changelog line in its report, version raise owed). The Deviation (the result type written in a typed let around the boxed future, since rustc carries no type through the `as` cast) was sound: made a non-deviation in `common.part` (a lowering rustc cannot take as the brief wrote it). CI: 818f6cdf green, later runs in progress. #938 re-probed (still fails for untyped number lists; `Vec[i64]` runs), commented, brief written. Batch: `b1031-kept-fn-name-copy.md` (Opus, unrun), `b938-loop-copy-inferred-list.md` (Opus, `Fixes #938`); serial, both in `check_*` files.

**2026-10-31 (architect after d7e56e49, cut at b1018 on Found):** #1018 done (d5995dbd, `Fixes #1018`; Changelog line in its report, version raise owed). Found widened by probes and filed `ready` in brief form as #1031 (a kept fn copied from a name, a field or a tuple part lowers to a move of the `Kept`; `keep(f)` to an owning parameter already clones). Unprobed aside: `f(1) + h.f(2) + k.f(3)` in one expression gave NK2605, `h.f(2)` alone runs. Batch: `b1029-kept-lambda-int-result.md` (Opus, unrun, unchanged), `b1031-kept-fn-name-copy.md` (Opus, `Fixes #1031`); both touch `check_facts.nika`, run serially.

**2026-10-31 (architect after 61df8cf5, cut at b1017 on Found):** #1017 done (ee9aa6a6, `Fixes #1017`; Changelog line in its report, version raise owed). CI red since 555fe6e9 (last green df143a1b) was `examples::every_example_package_passes_its_own_tests` (json ledger `signature`, stale after the lending of data parameters), fixed forward by 71aa29ae; every example passes `nikaia test --locked` locally, its run in progress. Found widened by probes and filed `ready` in brief form as #1029 (a pausing kept lambda typed `-> i64`/`u8`/`i64?` hands back i32 literals: the cast `as BoxedIn<'_, _>` in `emit_flow_kept`). Batch: `b1018-kept-fn-index-read.md` (unchanged), `b1029-kept-lambda-int-result.md` (Opus, `Fixes #1029`). b1012c waits on #1028 (`decision`). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #1014. Then: BT, GR, K3b, P5c (+`report`).

**2026-10-31 (architect after ed30efd5, cut at b1012b on Found):** #1012 step 2 done (51b0838a, NK1264/NK1265; Changelog line in its report, version raise owed). Found (`nikaia fmt` on a tools file rewrites it with tabs) is Part III's specified behaviour, not a defect: trap in builder-facts (never `fmt` tools files). Deviation (an existing test's stderr updated for a message the change rightly altered) made a non-deviation in `common.part`. CI green to df143a1b. Batch: `b1012c-trait-column-data.md` (now `Fixes #1012`, k3a2 is on develop), `b1017-lambda-optional-tail.md`, `b1018-kept-fn-index-read.md` (both probed; #1018's `fs[0](2)` row dropped, no Part I form). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #1014. Then: BT, GR, K3b, P5c (+`report`).

**2026-10-31 (architect after a3ec9600, cut at b1024 on Found):** #1024 done (ac8a38a6, `Fixes #1024`; Changelog line in its report, version raise owed). Found probed and filed `ready` in brief form as #1025 (a lent parameter a kept lambda names, no use after: the keeps walk reads the lambda body as running during the call, `keeps_blocks_in`, so rustc refuses; `spawn` is the model). CI to b2866e02 green, 0d053671..a3ec9600 running. Batch: `b1025-kept-lambda-names.md` (Opus, `Fixes #1025`), `b1012b-trait-column-refusals.md` (unchanged), `b1012c-trait-column-data.md` (precondition b1012b). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #1014. Then: BT, GR, K3b, P5c (+`report`).

**2026-10-31 (architect after b4938048, cut at b1023 on `partial`):** #1023 steps 1 and 3 done (3b6bf8c7, 90d7e2ad; Changelog line in its report, version raise owed with ADR-277 D11's sentence). Step 2 is the checker's (`a_lent_parameter` matches by name, a shadowing let reads as the lent parameter): not an owner decision, the architect allowed the checker files; comment on #1023; rule under *Briefs and reports* (a `&` at a call is the checker's). Batch: `b1023b-shadowing-lambda-lent.md` (Opus, `Fixes #1023`), `k3a2-comptime-compute.md` (precondition names b1023b), `b1012b-trait-column-refusals.md`. Next: `b1012c-trait-column-data.md`. Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #1014. Then: BT, GR, K3b, P5c (+`report`).

**2026-10-31 (architect after 32080dd6, cut at k3a2 on `blocked`):** fix-forward corpus (check_arms/check_core_flow), #994 and #1012 step 1 (1c72e8b9) done, Changelog lines in their reports, version raises owed. CI red since 28a0c1a2 on `touch_sets::the_two_console_handles_keep_their_order` (#994's text), fixed forward by 0b8a3acb, its run in progress. k3a2 run 4 found three lowering defects, probed and filed `ready` as #1023 (`Send + Sync` hits `tools`' `enum Sync`; a shadowing let lambda handed on unlent; a `fn` parameter on a call cycle instantiates `impl Fn` without end, decided: `&dyn Fn` on cycles, one sentence into ADR-277 D11 with the build). run4 patch tracked, report kept as `.run4.report.md`; builder-facts: `--features compiler` trap. Batch: `b1023-fn-param-cycle.md`, `k3a2-comptime-compute.md` (from run4.patch), `b1012b-trait-column-refusals.md` (all Opus). Next: `b1012c-trait-column-data.md` (`Fixes #1012` once k3a2 is on develop). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #1014. Then: BT, GR, K3b, P5c (+`report`).

**2026-10-30 (architect after 9ffc1ec2, RED arm64/bootstrap_compiler; batch b1007, b996 done):** #1007 (ad7f118f) and #996 (b3fe400c) done, Changelog lines in their reports, version raises owed; both close on promote. RED since the push ending 75e25ed8 (last green a5663bf6): `sequences::the_corpus_has_no_more_unanswered_method_calls_than_it_had` (ceiling 16); push held 43312d30 (yaml lane's prover change, +135 lines in `prover_walk.nika`), 7cde2178 (`packages/yaml` only, unreachable), a report: suspect 43312d30. The scan joined builder.md's guard set. A single arm64 `examples::every_example_package_passes_its_own_tests` failure at e89554fd passed again at a5663bf6 (flaky, watch). Batch: `b-fixforward-corpus-unanswered.md` (Opus; fix the checker gap, or `blocked` and then the architect picks fix brief or revert of 43312d30), `b994-overlap-note-seq.md` (Sonnet, `Fixes #994`). k3a2 still waits on the owner's #1012 answer (`docs/briefs/QUESTION`). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #1014 (cache key misses inferred `sync`). Then: BT, GR, K3b, P5c (+`report`).

**2026-10-30 (architect after a5663bf6, cut at b1007 on `blocked`):** #1008 done (d1ff2ec3, 0.0.1312). k3a2 waits on the owner: #1012 (`decision`, `ref fn(..)` for a lent callback; paper written, `docs/briefs/QUESTION` holds the question; run 3 saved as `k3a2-comptime-compute.run3.patch`). b1007 stopped: no checker fact carries a call's result type; brief rewritten with the fact (`summed` in `check_facts.nika`, written in `check_core_method.nika:568`), run 1's report kept as `.run1.report.md`; rule under *Briefs and reports*. #996 narrowed by probe: only the literal-text tier (`&'static str`) lacks `.copied()`; the owned value is refused NK1102 as D15 says. Batch: `b1007-sum-typed.md`, `b996-map-read-literal-text.md` (both Opus). After the owner's #1012 answer: its build brief, then k3a2 from the run3 patch. Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, `HashSet` union/intersection/difference (when met). Then: BT, GR, K3b, P5c (+`report`).

**2026-10-30 (architect after 3b845e0b, cut at k3a2 on `blocked`; RED was the empty state file, runs 8a30f8cb..3b845e0b in progress, no failed job):** k3a2's Found widened by probes and filed `ready` as #1008 (a trait method's `fn` parameter: the impl lowers `impl Fn` against the trait's `Kept`, E0049; a `Kept` handed on is moved, not cloned). k3a2's draft saved as `docs/briefs/k3a2-comptime-compute.run2.patch` (it lay in another session's /tmp), report renamed `.run2.report.md`, brief points at the patch and carries the run's notes. Rules: host forms probed before a move brief (*Briefs and reports*); builder.md's lower-std grep catches `Error:`. Batch: `b1008-trait-fn-param.md` (Opus, `Fixes #1008`), `k3a2-comptime-compute.md` (Opus), `b1007-sum-typed.md` (Sonnet). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check (RED on an empty file, again), #996, `HashSet` union/intersection/difference (when met). Then: BT, GR, K3b, P5c (+`report`).

**2026-10-30 (architect after 7ffa0921, cut at b1006 on Found):** #1006 done (d2986cd9, `Fixes #1006`; Changelog line in its report, version raise owed, with ADR-collections-2 D3). Found filed `ready` as #1007 (`sum()` of a typed number sequence lowers to a bare `.sum()`, rustc E0283/E0284; probes run, f64 too). b1006's debrief into builder-facts (`nikaia run` a scratch copy, no plain rustc, no sleep loop, contract-only methods). Batch: `k3a2-comptime-compute.md` (Opus, unchanged, all preconditions on develop), `b1007-sum-typed.md` (Sonnet, `Fixes #1007`). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #996, `HashSet` union/intersection/difference (when met). Then: BT, GR, K3b, P5c (+`report`).

**2026-10-30 (architect, owner answered k3a2's OWNER line: sets get `iter()`):** decision paper #1006 (`ready`, written after the fact; owner: never again a bare question), ADR-collections-2 D3 + Part III `std::collections` (both sets, `HashSet` by ADR-collections-1 D5; version raise owed with the build). Batch: `b1006-set-iter.md` (Opus, `Fixes #1006`), then a fresh `k3a2-comptime-compute.md` (first run's report kept as `.run1.report.md`, precondition names b1006). Rule added to the cycle's step 4 (paper before an OWNER question; re-queue after the answer). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #996, `HashSet` union/intersection/difference (ADR-collections-1 D5 defect, when met). Then: BT, GR, K3b, P5c (+`report`).
**2026-10-30 (architect after f2599c1c, cut at b998 on `Pushed: rejected`, false: f2599c1c is develop's tip; `drive_commit` looks only in `origin/develop..HEAD`):** #997 done (7029fe65, `Fixes #997`), #998 done (f2599c1c, `Fixes #998`); Changelog lines in both reports, version raises owed. b998's Found filed `ready` as #1005 (`.to_string()` of an owned local `String` or an untyped number local read as a view: NK1104/NK2302; probes run). Batch: `b-drive-pushed-on-develop.md` (Sonnet; log over `HEAD` and `origin/develop`, last 40), `b1005-to-string-owned-local.md` (Opus), `k3a2-comptime-compute.md` (Opus; precondition now names #1005). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #996. Then: BT, GR, K3b, P5c (+`report`).
**2026-10-30 (architect after 252ca959, cut at b995 on Found + Deviation, a real find: the brief's own lambda test row, unprobed; rule under *Briefs and reports*):** k3a1 committed (5bf28f57, Changelog none), #995 done (0cf0b173, `Fixes #995`; Changelog line in its report, version raise owed). Found filed `ready`: #997 (let-bound lambda with untyped `String` param lowers `|n| known(&n)`, E0277; and its map-read tail misses `.copied()`; blocks K3a part 2 at comptime_run.rs:393) and #998 (NK2302 on `match` over an owned enum whose arms hand back `to_string()`/`clone()`; views.nika `rsrc_*`). CI on 5bf28f57..252ca959 running. Batch: `b997-let-lambda-string-param.md`, `b998-match-owned-text-nk2302.md`, `k3a2-comptime-compute.md` (all Opus; k3a2's precondition now names b997). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #996. Then: BT, GR, K3b, P5c (+`report`).
**2026-10-30 (architect, cut at k3a1 on a missing `Pushed:` line: the builder wrote `Result: done` with placeholders and committed nothing; rule in common.part: commit and push before the report):** k3a1's work lies uncommitted in the main checkout (comptime_tree.nika + lowering + index hunk, comptime_run.rs, rs2nika.names). Filed #995 (`a ?? m.get(k)` with owned `a`, Option<T> vs Option<&T>, `ready`) and #996 (`return m.get(k)` as `String?` from a lent map, `ready`). Batch: `k3a1-commit.md` (Sonnet: verify, re-lower, commit), `b995-coalesce-map-get.md` (Opus; K3a part 2 needs `known(n) ?? ahead.get(n)`), `k3a2-comptime-compute.md` (Opus; `computed`, `worked_out_ahead` behind `ComptimeComputeHost`, `compute` stays the Rust adapter). Sets lack `iter`/`filter` in std.contracts (no ADR): k3a2 uses `difference`; a decision paper is owed if a move needs more. Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check, #996. Then: BT, GR, K3b, P5c (+`report`).
**2026-10-29 (architect after 7b36e23, cut at b933 on a Deviation, a false cut: the brief wanted `-> String`, the builder followed ADR-353 D10's caller keep; rules in common.part, *Briefs and reports* and builder-facts (`--tethers`, `nikaia.contracts`, no `nikaia test` filter)):** #933 done (34519e7, `Fixes #933`; Changelog line in its report, version raise owed). CI on 22d4a9e..7b36e23 running. Batch: `p5e-project-tag-copy.md` (Opus, precondition met), `k3a1-comptime-lower.md` (Opus; K3a split: part 1 `lowered_units` .. `by_package` behind `ComptimeLowerHost`, part 2 `compute` .. `culprit` next over that host). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check. Then: K3a part 2, BT, GR, K3b, P5c (+`report`).
**2026-10-29 (architect after e9fda67, cut at b934 on a Deviation, a false cut: the brief's sketched `.stdout` lacked the cleanup wrapper; rule in common.part):** #934 done (b1759a1, `Fixes #934`; Changelog line in its report, version raise owed). Batch: `b933-method-result-owned.md` (Opus, unchanged), `p5e-project-tag-copy.md` (Opus; the P5a rest: `program_name`, `root_tag`, `own_copy`, `trace`, faithful, two host methods `sha256_hex`/`hard_link`; blocked if #933 is not on develop). Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check. Then: K3a, BT, GR, K3b, P5c (+`report`). Untracked `k1-comptime-tree.*` and the uncommitted `emit_*`/`keeps`/`text_tiers`/`rs2nika/convert/nikaia.*` are another session's.
**2026-10-29 (architect after ed019bb, cut at b943 on Found):** fix-forward 7c1643d (formatter) and #943 (417fc04, Fixes #943) done, Changelog lines in their reports. The cut was 7c1643d's own test: its `.stdout` had a newline `print` does not write (red bootstrap_compiler + arm64); fixed by the architect in 6ccf6c8, rule into builder-facts (new test run through `nikaia test`). Batch: `b934-closed-by-hand.md` (checker, NK2605), `b933-method-result-owned.md` (lowering, `views`/`text_tiers`), both Opus; probes re-run and #933's corrected (`ref self`). They free the P5a rest (`root_tag`, `program_name`, `own_copy`, `trace`), which is the next batch, with `Cut:`/`Convert:`/`Shape:`. Still owed: R0 (#921), `scripts/plan_rows.nika`, the driver's CI check. Then: K3a, BT, GR, K3b, P5c (+`report`). Untracked `k1-comptime-tree.*` and `crates/rs2nika/convert/nikaia.*` changes are another session's.
**2026-10-29 (architect after 5d90658, batch done; CI red at 0f9a525):** P5d (0f9a525, 82.2 %) and P5b (6a65c91, kept 11 of 94) done, Changelog none. CI red: `fmt::every_nika_file_...` (bootstrap_compiler, arm64), the formatter (`tools/fmt.nika`) cuts a nested tuple-pattern closure in `project_check.nika:440`; minimal probe in the brief. `fmt::every_nika_file` joined builder.md's guard set; builder-facts got the `[forms]` matching rule (Nikaia draft text, P5b) and `map_clone`. Batch: `b-fixforward-fmt-tuple-closure.md`, `b943-literal-local-push.md` (both Sonnet). Untracked `k1-comptime-tree.*` in `docs/briefs` is an old run's (7e3f8eb), not mine. Still owed: P5a rest after #933/#934 (lane briefs), R0 (#921), `scripts/plan_rows.nika`, the driver's CI check. Then: K3a, BT, GR, K3b, P5c (+`report`), each needing a brief with `Cut:`/`Convert:`/`Shape:`.
**2026-10-29 (architect after 02b36d6, cut at m3b on Found):** MF (eb90dc3, Changelog: manifest line in its report) and M3b (093923c, kept 5 of 73, Changelog none) done. Found filed as #943 (`ready`, brief form: `let mut out = "a"` + `push_str` is `String?` under `lower-std`); M3b's debrief went into builder-facts (*Host traits*, anchored `it` filter, typed text literal). Batch: `p5d-project-check.md` (`check`, `newly`, `keeps_that_moved`, `explain`; `report` moved to P5c, it needs P5b's program host), `p5b-project-lower.md` (`lower_reading` only, narrowed). Both `Shape: schematic host-calls`: step 1 adds `[forms X_]`/`[hosts X_]` rows to `rs2nika.names` and re-converts, so the draft carries the host calls. Disk 3.7 GB free (agent.nika escalates under 3). Still owed: P5a rest after #933/#934, R0 (#921), `scripts/plan_rows.nika`, the driver's CI check. Then: K3a, BT, GR, K3b, P5c (+`report`).
**2026-10-29 (architect, process note from the owner):** moves are faithful by default; `Shape: faithful | schematic <patterns> | redesign <reason>` in every move brief; a redesign gets no `Convert:` line and no 60 % rule; five schematic patterns wanted from the converter, in order, under *Briefs and reports*. `drive.nika` unchanged (it already checks only a `kept N of M`). Next move briefs (P5d, M3b, P5b) take the `Shape:` line.
**2026-10-29 06:40 (architect after 06eaea6, cut at m3a on Found, a false cut: another session reset the shared tree at 22:41, a process event; rule in CLAUDE.md, builder.md (re-check lowered files before long tests) and common.part (such an event goes under `Needs:`)):** M3a done (1890fc9, 82.1 %, Changelog none). Converter defect for the converter lane: `--prefix` not applied to functions and types (m3a's costliest edit). Batch: `mf-manifest-file.md` (Opus, unchanged but for the new rules block), `m3b-modules-program.md` (Opus; `of` + `emit_reading` behind two hosts with signatures). Still owed: P5a rest after #933/#934 (lane briefs), R0 (#921, C1 not quiet: `crates/rs2nika/convert/nikaia.*` uncommitted), `scripts/plan_rows.nika`, the driver's CI check. Then: P5d, P5b.
**2026-10-28 22:40 (architect after 3ba472a, cut at p5a on `partial`; the RED line was the empty state file: CI green to 986390f, 2e13458/3ba472a running):** P5a built in part (2e13458, 82.1 %, Changelog none): `root_tag`, `program_name`, `own_copy`, `trace` wait on two defects, filed `ready` in brief form: #933 (lowering: a host method's `String` result, viewed, lowers as `&'r str` of the parameters; brief `lane-lowering-933-host-result-view.md`) and #934 (NK2605 after an explicit `close() catch`; brief `lane-bugs-934-close-by-hand.md`); both C3 lane briefs, not in `batch.txt`. Debrief fixed at the source (builder-facts: own copy of the binary for long tests; a method-shaped draft for free functions). Batch: `m3a-modules-collect.md` (Opus, unchanged), `mf-manifest-file.md` (Opus; `rooted` stays Rust). Still owed: R0 (C1 not quiet: `crates/rs2nika/convert/*`, `text_tiers.nika` uncommitted), `scripts/plan_rows.nika`, the driver's CI check, the P5a rest after #933/#934. Then: P5d, M3b, P5b.
**2026-10-28 22:10 (architect after 986390f, cut at b-fixforward-sweep2 on a Deviation, a false cut: re-keying `par_fold`'s KNOWN entry after 79d6e81, same program and reason; rule added to common.part):** sweep fixed forward (1f3adf2: `fn main` asked of the parsed items, not the text; Changelog none); CI runs 60b1154..986390f in progress. `project.rs`/`modules.rs` unchanged since a4b33a0, so the two move briefs stand. Batch: `p5a-project-settings.md`, `m3a-modules-collect.md` (both Opus). Still owed: R0 (C1 not quiet: converter lane's `crates/rs2nika/convert/*` uncommitted), `scripts/plan_rows.nika`, the driver's CI check missing a red sweep step. Then: P5d, MF, M3b, P5b.
**2026-10-28 21:50 (architect after 3719095, cut at b-fixforward-sweep on `partial`):** the sweep step now annotates (60b1154, CI only, Changelog none); its run was in progress. Cause narrowed without logs: c7e5b66's push also carried eeaefb1 (0.0.1258, keep + a second `Get` impl for `HashMap` in `index.rs`), which had no run of its own; c7e5b66 itself cannot reach the sweep. Rule added under *Briefs and reports* (list every commit of the first red push). Batch: `b-fixforward-sweep2.md` (Opus, three suspects in order), `p5a-project-settings.md`, `m3a-modules-collect.md` (unchanged). Still owed: R0 (C1 not quiet), `scripts/plan_rows.nika`, the driver's CI check missing a red sweep step. Then: P5d, MF, M3b, P5b.
**2026-10-28 21:40 (architect after a783081, batch done; CI red unseen):** K1+K2 done (7e3f8eb), lane-1 plan done (eac4415, *Large files 3*), both Changelog none, no Found. CI on `develop` red since c7e5b66 (16:14, converter `--engine tree`): arm64 *Accepted-programs sweep* fails on every run whose `Test` passed; last green 590cd7c; the driver said BATCH DONE, not RED (the RED file was empty: the driver's CI check misses a red sweep step, look at it when the fix-forward is in). Job logs are proxy-blocked and the sweep writes no annotations, so the cause is unknown: `b-fixforward-sweep.md` reproduces it narrowly and makes the step annotate. Batch: `b-fixforward-sweep.md` (Opus), `p5a-project-settings.md` (Opus, four callbacks: env_var, native_cpu, sha256_hex, hard_link), `m3a-modules-collect.md` (Opus). R0 still waits: C1 is not quiet (converter lane's `crates/rs2nika/convert/nikaia.{contracts,derived,proofs}` uncommitted). Process item owed (p5-plan debrief): `scripts/plan_rows.nika -- FILE:RANGES PREFIX` (cut, wrap in impl, convert, print share and TODO reasons) for planning briefs. Then: P5d, MF, M3b, P5b per *Large files 3*'s order.
**2026-10-28 14:15 (architect after a29d3b8, batch done):** P4b (8394790, 80.8 %) and P1 (aab4d74, 80.9 %) done, both Changelog none, no Found. Batch: `k1-comptime-tree.md` (Opus, K1+K2 together, ~565 lines; `Computed` becomes a re-export of `CheckComputed`, lane 2's `contracts/*`/`check/*` untouched), `p5-lane1-plan.md` (Opus, docs only: rows P5, M3, MF, K3, BT, GR with `Cut:`/`Convert:` lines; the old *Stays Rust* drivers are this week's work). R0 is due (#557 steps 1-2 on develop, 2ca9841/9545797) but C1 is not quiet: the converter lane's `crates/rs2nika/convert/src/main.nika` was being edited at 14:06, which R0's step 1 refuses; the supervisor runs R0 between K1 and P5 or after them once that file is committed (both briefs read `crates/nikaia-compiler/src` if R0 ran). P1's debrief rule (reshaped struct: its readers with counts) is under *Briefs and reports*. Then: P5/M3 from the new table, S1 (Fri), K3.
**2026-10-28 (supervisor, owner's deadline):** until 2026-11-04 the plan is `docs/bootstrap-week.md` (owner: a complete bootstrap, ADR-294 D5 Stage 2, by 2026-11-04; careful parallelism: two move lanes, a lowering lane, a compiler lane). Its day targets, lanes, `Owns:` lists and the `lower-std` rule bind every brief; lane briefs (`lane2-*`, `lane-lowering-*`, `lane-bugs-*`) never go into `batch.txt`. The driver is lane 1.
**2026-10-28 (architect, owner's corrected goal):** `docs/bootstrap-week.md` sections 1-2 and 7 rewritten: done = the compiler (adapters <= 2,950) and `crates/orchestrator` (all moves, crate deleted, #923 `blocked` on #920/#922/#921); `crates/nikaia-std` out of scope, only new entries #920 (rewritten: per-stream, stdin) and #922 (`ready`: sha256, `env`, `fs::lock`, file times), D-7 asks the owner to confirm. R0 relocation `crates/nikaia-std/src/tools` -> `crates/nikaia-compiler/src` (#921 `ready`, the text of ADR-294's new D21 in `docs/briefs/r0-compiler-crate.md`), once in C1 at a quiet moment after P4a and #557a; not in `batch.txt`. D-5 dropped, D-6 cut order: process items, then O2/O1, never R0 or Stage 2. S-B 19,210 lines, 2,710 a day.
**2026-10-28 (architect after 905bee7, cut at p4a on Found):** P4a done (3620d27, 80.7 %, Changelog none). Found (`Seq` has no `chain`) filed as #924 (`ready`, brief form), widened to `is_ascii_alphabetic`/`is_ascii_alphanumeric`; waits until after 11-04 (std out of scope), so P1's `symbol_prefix` stays Rust. The debrief's two points were already built in 905bee7. Batch: `p4b-project-foreign.md` (Opus; now with `Cut:`/`Convert:` lines at 905bee7), `p1-project-settings.md` (Opus; its ranges lie above P4b's, `encoded_with_cpu` below them moves by hand). R0 still waits for #557 steps 1-2 (none on develop). Then: K1+K2 (`comptime_run.rs`), P5 (`project.rs` build drivers), both per the Thu row.
**2026-10-28 (architect after c1ddc12, cut at p3b on Found):** P3b done (5681395, 80.6 %, Changelog none); p4a did not run. Found filed as #916 (`ready`, brief form: `ref Bytes?` argument opened with `.as_ref()`, `.clone()` of a `Bytes` view is a `Vec<u8>`, a match over a cloned option compared with a `ref` binding); not batched (blocks no move: written around by matching the field, P4b told so). Batch: `p4a-project-members.md` (Opus, unchanged), `p4b-project-foreign.md` (Opus; sha256 and the `LedgerOps` calls as a host trait, S2's `SysrootHost` precedent; blocked if P4a is not on develop). Then: K1 after the converter's *collect into an Option* (still a TODO, R150); S1/C1 drafts 44-45 %, wait for the converter; E13-1 rides with C1 or `with_contracts`; #907, #908, #909, #916 only when a move needs them; #882 stays the main-session lane's.
**2026-10-28 (architect after 33f9df4, batch done):** M2 done (2000485, 80.4 %), S3 done (f861426 std, 594cb42 `shown_path`; raised in 0.0.1255), both Changelog none in their reports. CI green to 09145fb, f861426..33f9df4 running. P4 split: P4a members (`project_members.nika`; `rooted` stays Rust, it is `toml::Value`), P4b `Foreign` .. `description_at`. std checked for both: `process::run` cannot replace `outcome_of` (no stdin bytes, no env), so the spawns stay Rust as callbacks. Batch: `p3b-project-run-loop.md` (Opus), `p4a-project-members.md` (Opus; its lines lie above P3b's, so P3b does not shift them). Then: P4b, K1 after the converter's *collect into an Option*; S1/C1 wait for the converter; #907, #908, #909 only when a move needs them; #882 stays the main-session lane's.
**2026-10-28 (architect after 60ce1f9, cut at p3a on a Deviation, a real one but the brief's fault: it moved `shown_path`, whose `canonicalize`/`strip_prefix` `std` lacks; rule added under *Briefs and reports*):** P3a done (b8547a6, 80.3 %, Changelog none). CI green to 025f3d5, 60ce1f9 running. Decided (ADR-319 D3 amendment, text in the brief): `Path.strip_prefix -> Path?` and `fs::canonical(path, root) -> Path throws`. Batch: `m2-modules-tests.md` (Opus, unchanged), `s3-fs-canonical.md` (Opus; std, version raise owed; then `shown_path` into `project_tests.nika`). Then: P3b, P4 (both need S3), K1 after the converter's *collect into an Option*; S1/C1 wait for the converter; #907, #908, #909 only when a move needs them; #882 stays the main-session lane's.
**2026-10-28 (architect after 67dfd5a, cut at s2-sysroot-cache on Found; CI green to 15ef355):** M1 done (3136e7b, 80.1 %), S2 done (9826a9e, 80.2 %; probe 070b9c7, fixes 337e702, 198ded4), both Changelog none. Found filed as #909 (`ready`, brief form: lent loop element moved into a tuple literal passes the checker, rustc E0308); not batched (blocks no move). P3 split: P3a discovery/verdicts now, P3b the run loop (`run_tests`, `outcome_of`, `run_program_test`, `bless*`, `explain_output`) next. M2's intern prerequisite settled: callback as `check_walked_kept_call`. Batch: `p3a-project-tests.md` (Opus), `m2-modules-tests.md` (Opus). Then: P3b, P4 (after P3a), K1 after the converter's *collect into an Option*; S1/C1 wait for the converter; #907, #908 (converter lane's finds) and #909 only when a move needs them; #882 stays the main-session lane's.
**2026-10-28 (architect after fa57fcc, batch done):** P2 done (e51e3e2, 80.0 %, Changelog none); p2 plan done (6dc116d: rows M1, S2, M2, S1, C1, E13-1 in *Large files 2*). CI on bcb60d1..fa57fcc running, earlier green. Batch: `m1-modules-program.md` (Opus), `s2-sysroot-cache.md` (Opus; builds the `std::fs` bytes probe P3/P4 wait for; cache keys must not change: Rust passes its `Debug` strings in). Converter lane (not briefed by the architect): R58 marks `Ledger` as Rust-only though it is `ty.nika`'s; S1 and C1 wait for `.get(range)` on `String`, `else if let`, or-patterns with `Some` (#886), `.entry().or_default()`. Then: M2 (with or after M1), P3/P4 after S2's probe, K1 after the converter's *collect into an Option*, #557 for the parser rows; #882 stays the main-session lane's.
**2026-10-28 (architect after ef2ce14, cut at b905 on a Deviation, a real one but the brief's fault: its `for`-push row contradicted ADR-094 D4; rule added under *Briefs and reports*):** #905 done (e3b8c74, `Fixes #905`; Changelog line in its report, version raise owed). CI on b4fd52c..ef2ce14 running. P2 briefed from p1's table (`check::Finding` is `CheckFinding` of `check_state.nika`, not `findings.nika`'s). p2-plan retargeted to `sysroot.rs`, `modules.rs`, rest of `emit/mod.rs` (p1 planned `comptime_run.rs`). Batch: `p2-project-check-summary.md` (Opus, move), `p2-large-files-plan.md` (Opus, docs only). Then: K1 needs the converter's *collect into an Option* first (converter lane); #557 brief (span/here/same_line) for the parser rows; P3/P4 after the `std::fs` bytes probe; #896 steps 3-4, #898, #899, #895 only when a move needs them; #882 stays the main-session lane's.
**2026-10-28 (architect after 42c0246, cut at n903b on Found; `RED` was the empty state file, CI on 00451bf..42c0246 running):** #903 done (ee19152, `Fixes #903`; Changelog line in n903b's report, version raise owed). Found widened by probes and filed as #905 (`ready`, brief form): a variant's `Vec[String]` part bound in a `match` over an owned value is typed `Vec[ref String]` (return / hand-on / `for` refused, `let r = match` reaches rustc); in tools a drained part gets an unused `let mut`; `node_numbers.nika` carries a workaround to undo. Batch: `b905-variant-list-part.md` (Opus, `Fixes #905`), `p2-large-files-plan.md` (Opus, docs only). Then: brief #557 (trailing-trivia cut, unblocks the grammar rows of the parser slice plan), moves from the p1/p2 tables, two per batch from different files; #896 steps 3-4, #898, #899, #895 only when a move needs them; #882 stays the main-session lane's.
**2026-10-28 (architect after 51efe15, RED + cut at n903a on a Deviation, a real one: the brief said "read issue #903", builders have no GitHub):** #903 steps 1-2 done (39ebd83, `Part: #903`, Changelog none). RED since c6626c2 (0.0.1251 left the spec date at 2026-10-27, `the_specifications_version_is_the_changelogs`) is already fixed forward by 43942ee (its run was in progress). n903b now carries #903's text, the n903a API facts and locations (120 turns). Batch: `n903b-node-numbers-pass.md` (Opus, `Fixes #903`), `p2-large-files-plan.md` (Opus, docs only). Then: the grammar rows of the parser slice plan once #557 is built (brief #557 next), moves from the p1/p2 tables, two per batch from different files; #896 steps 3-4, #898, #899, #895 only when a move needs them; #882 stays the main-session lane's.
**2026-10-27 (architect after d8264d0, cut at e12-4b-target on Found, a false cut: a cut input, not a converter defect, handled by e421d39 R216):** E12-4b done (d1ea072, 79.6 %, Changelog none). Rule in common.part: a wrong converter draft goes under `Rs2nika:`, not `Found:`. CI on 3198a2e..d8264d0 running (f348b00 green). No planned move is left, so the batch plans: `p1-large-files-plan.md` (Opus, `project.rs`, `parser/mod.rs`), `p2-large-files-plan.md` (Opus, `comptime_run.rs`, `sysroot.rs`, rest of `emit/mod.rs`); both docs only, append to `selfhosting-plan.md`. Then: two moves per batch from those tables, from different files; #896 steps 3-4, #898, #899, #895 only when a move needs them or develop is red; #882 stays the main-session lane's.
**2026-10-27 (architect after d6318de, cut at b896a on Found; the RED stop line was stale: the last red runs are 17:46, green after, d6318de.. running):** #896 steps 1-2 done (3419c1d, d724ac0, `Part: #896`; Changelog line in its report, version raise owed). Found filed as #899 (`ready`, brief form: `mut` argument read by another argument; mismatched_lifetime_syntaxes). Owner's direction (`priorities.md`, c021392/f4b01e3): moves first, largest Rust files first; a bug only when it blocks a move or turns develop red. So #898 and `e12-4b0` leave the batch (#898 no longer blocks: R205 writes a `match`, 2941876; e12-4b0 is not needed: `Build` stays Rust, its `Default` holds `env!("NIKAIA_HOST_TRIPLE")`, which no field default can, and the orphan rule forbids the impl in `crates/nikaia`). Batch: `e12-4b-target.md` (Opus, `Target` + `BoundsChecks`/`OverflowChecks` into `emit_build.nika`), `p1-large-files-plan.md` (Opus, docs only: slice table for `project.rs`, `parser/mod.rs` with converter draft shares). Then: two moves per batch from that table; #896 steps 3-4, #898, #899, #895 when a move needs them or the pause rule stops the lane; #882 stays the main-session lane's.
**2026-10-27 (architect after 14869e3, cut at b889 on a Deviation, a false cut: the emit fix was already on develop, 30798b2):** #897 done (68f83b0, f76c4d8, `Fixes #897`; Changelog line in its report, version raise owed); #889 test-only (ea62f5b, `Fixes #889`). Rule in common.part: a step another commit already built, shown by the test failing with it reverted, is no deviation. New `ready` #898 (converter lane's find: `ref T?` not narrowed, NK1102) briefed. E12-4b split: its prerequisite `impl Default` for an all-default struct (probe: none today) is `e12-4b0`. Batch: `b896a-slice-in-tuple-to-string.md`, `b898-ref-nullable-narrowed.md`, `e12-4b0-struct-default-impl.md` (all Opus). Then: #896 steps 3-4, E12-4b move, #895 if the converter lane has not taken it, #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after 5ce6a67, cut at b894 on Found):** fix-forward range guard (65d038d), #893 (34fe82f), #894 (b82ab26, `Fixes #894`) done; Changelog lines in b893/b894 reports, version raise owed. CI green to 30d2095, b82ab26.. running. Found filed as #897 (`ready`, brief form: text `+` of a lent field moves it; an owned local in an arm of a lent argument is moved). #896 (converter lane's find, compiler defects) split: steps 1-2 now, 3-4 (std gaps, `filter_map` `Some`) later. Batch: `b897-lent-field-plus-and-arm-local.md`, `b889-guard-tuple-arm-by-value.md`, `b896a-slice-in-tuple-to-string.md` (all Opus). Then: #896 steps 3-4, E12-4b (Default lowering first), #895 if the converter lane has not taken it, #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after f571dcf, RED, cut at b893 on blocked):** RED since f74c199 (converter lane, R177): `range_spelling` guard takes `".."` inside string literals of a `for` head (`rs2nika/convert/src/main.nika`) for an old-spelling range; fix-forward in the guard (`b-fixforward-range-guard.md`, Sonnet), not in main.nika (another lane's uncommitted work). b893 built and tested #893 but left it uncommitted because of the red gate; `b893-commit.md` (Sonnet) commits those four files. Batch: `b-fixforward-range-guard.md`, `b893-commit.md`, `b894-lent-argument-arms.md` (Opus, unchanged). Then: #889, E12-4b (Default lowering first), #895 if the converter lane has not taken it, #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after bb3a9df, cut at e12-4a on Found):** E12-4a done (ae04379, 79.5 %; Changelog none). Found filed as #895 (`ready`, brief form: rs2nika `--file` drops a `const` slice silently; converter lane, the main session's, not in this batch). The other lane's uncommitted files named in b893/b894 are gone from the checkout (clean); the lines stay harmless. Batch: `b893-optional-let-proved-arm.md`, `b894-lent-argument-arms.md` (both Opus, unchanged). Then: #889, E12-4b (Default lowering first), #895 if the converter lane has not taken it, #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after bc5856d, cut at b892 on Found):** fix-forward agent_script cap done (a8cb0c5); #892 done (74ff66f, `Fixes #892`; Changelog line in its report, version raise owed). Found filed as #894 (`ready`: a lent argument given an `if`/`match` mixing a `ref` view arm and a new value lowers without `&` on the new arm; rustc Mismatched types; edition 2024 needs the new value hoisted). Uncommitted `emit_core_calls.nika` + `crates/rs2nika/convert/**` in the checkout belong to another lane: every brief names them as not its own. e12-4a's report block refreshed (`Rs2nika:` field). Batch: `e12-4a.md` (Opus, first so it finally runs), `b893-optional-let-proved-arm.md` (Opus, `Fixes #893`), `b894-lent-argument-arms.md` (Opus, `Fixes #894`). Then: #889, E12-4b (Default lowering first), #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after 7eba69e, RED, cut at b890 on Found):** #890 done (f6bba78, `Fixes #890`; Changelog line in its report, version raise owed). RED: `agent_script` test still expects ESCALATE at 200.2 USD after 26db5c6 raised the day's cap to 1000 USD; fix-forward `b-fixforward-agent-cap.md` (Sonnet). Batch: `b-fixforward-agent-cap.md`, `b892-arm-moved-value.md` (Opus, `Fixes #892`), `e12-4a.md` (Opus, unchanged). Then: #893, #889, E12-4b (Default lowering first), #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after cd6d3d8, cut at b888 on Found):** #888 done (3b77d7c, `Fixes #888`; Changelog line in its report, version raise owed). Found filed as #892 (`ready`: a struct moved in an `if`/`match` arm bound by `let` and read later passes the checker, rustc refuses; `hands_over` skips arm tails) and #893 (`ready`: `let s: T? = if b != null { b } else { c }` Mismatched types). Batch: `b890-text-literal-tuple-pattern.md` (Opus, unchanged), `b892-arm-moved-value.md` (Opus, `Fixes #892`), `e12-4a.md` (Opus, unchanged). Then: #893, #889, E12-4b (Default lowering first), #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after 993664a, batch done):** #886 (81310c8), #891 (3ba8582) done, raised in 0.0.1243/0.0.1244; E12-3b done (0e3fea0, 79.1 %). #888 narrowed by probe: only the `let`-bound `if` form fails (rustc); the `match` form runs; a lent parameter in those forms is #882's. #890 step 3 decided yes (a literal is the same part; ADR-352 D1 amended, version raise owed with the build). E12-4 split 4a/4b (plan): 4b needs a lowered `impl Default` from field defaults. Batch: `b888-let-bound-proved-name.md`, `b890-text-literal-tuple-pattern.md`, `e12-4a.md` (all Opus). Then: #889, E12-4b (Default lowering first), #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after d25c5b9, cut at e12-3a on a Deviation, a false cut: steps 1+2 in one commit):** E12-3a done (33b5fe7, 79.0 %); b-commit-brief-name done (a6d3bd8). Rule in common.part: two steps in one commit, all built, is no deviation. #891 reproduces (tuple part only; struct field passes). E12-3b decided: `place`/`floor` go into `EmitWalk` (EmitState is read-only), host keeps only `seeds_shape`. Batch: `b886-struct-field-narrowing.md` (Opus, `Fixes #886`), `b891-lent-tuple-part-coalesce.md` (Opus, `Fixes #891`), `e12-3b.md` (Opus). Then: #888, #889, #890 (literal as same part, ADR-352 D1, architect's call), E12-4, #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after 43145d6, cut at b887 on Found):** #887 done (6e7816c, `Fixes #887`; Changelog line in its report, version raise owed); b-drive-restart done (ae188e4). Found filed as #891 (`ready`, `??` on a part of a lent tuple parameter moves it). New `ready` bugs #889 #890 (rs2nika lane L); #890 asks whether a literal counts as "the same part" (ADR-352 D1) - architect's call, open. Batch: `b-commit-brief-name.md` (Sonnet; commit.nika finds `<brief>.md` from `.md.report.md`, `Body:` field), `e12-3a.md` (Opus, unchanged, put before the bug so it finally runs), `b886-struct-field-narrowing.md` (Opus, `Fixes #886`). Then: #888, #889, #890, #891, E12-3b, #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after 54cbba0, cut at b885 on `Pushed: rejected`, false: d942502 is on develop):** #885 done (d942502, `Fixes #885`; Changelog line in its report, version raise owed). Cause of the false cut: the driver running since 10:52 predates 5a91e92 (`nikaia run` compiles `drive.nika` once); brief `b-drive-restart.md` makes it stop with `RESTART` (exit 3) when its source changed. New `ready` bugs #886 #887 #888 (from the #879 builder). b885 noted that `scripts/commit.nika` has no body field and misses the Attribution under the `.md.report.md` name (not briefed yet). Batch: `b-drive-restart.md` (Sonnet), `b887-tuple-nullable-wrap.md` (Opus, `Fixes #887`), `e12-3a.md` (Opus, unchanged). Then: #886, #888, E12-3b, #884 / #851 decisions; #882 stays the main-session lane's.
**2026-10-27 (architect after a0b8395, cut at b-drive-rejected-push on `Pushed: rejected`, a false cut: 5a91e92 is on develop; the running driver still had the old `drive_commit`, the next run has the rewrite):** the rejected-push process point is built. #885 decided (a statement `catch` drops the value, not NK1103) and briefed. Batch: `b885-catch-statement.md` (Opus, `Fixes #885`), `e12-3a.md` (Opus, unchanged). Then: E12-3b, #879 decision round, #868 move, #851 `decision`; #882 stays the main-session lane's.
**2026-10-27 (architect after 52288c3, cut at b883 on `Pushed: rejected`, a false cut again: f95b026 is on develop):** #883 done (f95b026, `Fixes #883`; Changelog line in its report, version raise owed). The open process point is now a brief: `b-drive-rejected-push.md` (Sonnet; `drive_commit` rewrites `Pushed: rejected` when its own push carries the builder's commit). Batch: `b-drive-rejected-push.md`, `e12-3a.md` (Opus, unchanged). Then: E12-3b, #879 decision round, #868 move, #851 `decision`; #882 stays the main-session lane's.
**2026-10-27 (architect after 5b2b3ca, cut at b881 on Found):** #881 done (0fa94bb, `Fixes #881`; Changelog line in its report, version raise owed). Found filed as #883 (`ready`: reassigned `String` local whose trimmed view becomes the result goes through `put_viewed`). CI on 0fa94bb/5b2b3ca was running; 3cb9fff's red was followed by green runs. Batch: `b883-reassigned-trim-result.md` (Opus, `Fixes #883`), `e12-3a.md` (Opus, unchanged). Then: E12-3b, #879 decision round, #868 move, #851 `decision`; #882 stays the main-session lane's.
**2026-10-27 (architect after 3cb9fff, cut at e12-2 on `Pushed: rejected`, a false cut: its commits cfad8bb, 4296d9b are on develop):** #880 done (ae26238c; Changelog line in its report, raised in 0.0.1232). E12-2 done (78.6 %). The plan's E12-3 prerequisite was wrong (maps `Shape` is no twin of `ty.nika`'s sequence `Shape`): corrected, E12-3 split 3a (read walks) / 3b (host `seeds_*`, `place_*`, `floor`). #882 is taken by a main-session lane (do not brief). Batch: `b881-empty-string-local.md` (Opus, `Fixes #881`), `e12-3a.md` (Opus). Then: E12-3b, #879 decision round, #868 move, #851 `decision`. Open process point: `batch.nika` cuts on `Pushed: rejected` although the driver rebases and pushes the run's commits; it should rebase-and-push first and cut only if that fails.
**2026-10-27 (architect after bae4710, cut at e12-2 on `blocked`):** #878 done (3fc712c, Changelog 0.0.1230 line in its report, version raise owed). E12-2 asked for a fourth host method: approved (`EmitHolesHost` for `Parsed`, `wrapped_hole(at, hole) -> Expr?`; the only `hole_as_read` caller takes `.into_owned()`), written into `e12-2.md`; first run's report kept as `e12-2.md.run1.report.md`. #879 relabelled `decision` (guard / tuple-position narrowing; owner round owed, R95 waits). Batch: `b880-string-move-view.md` (Opus, `Fixes #880`, then cherry-picks R92 7881ad4 for #877), `e12-2.md` (Opus). Then: #879 decision round, E12-3 (drop the `contracts/maps.rs` `Shape` twin first), #868 move, #851 `decision`.
**2026-10-27 (architect after 2cc30a9, batch cut at b869b on a Deviation, a real one but harmless):** #869 done (441382d, a9978f9, 65e011e `Fixes #869`; Changelog line in its report, version raise owed with #872-#874's). Not red: the `RED` seen was the empty state file `docs/briefs/RED`; CI on 441382d..2cc30a9 was running. The Deviation (parser refuses `if let`/`is` but says `let_`) filed as #878 (`ready`). ff8d00d (guide Memory) checked: no workaround visible, nothing owed. Batch: `b878-condition-binding-message.md` (Sonnet, `Fixes #878`), `e12-2.md` (Opus). Then: E12-3 brief (drop the `contracts/maps.rs` `Shape` twin first), #868 move, #851 `decision`.
**2026-10-27 (architect after 6920907, cut at b869a on a summary after `Result:`, a false cut):** #869 steps 1/2/4 done (6f72dfa, f8c4bb6; Changelog line in its report, version raise owed); #872-#874 done (b872 report, three Changelog lines, version raise owed); E12-1 done (654866b, 78.3 %). Rule in common.part: `Result:` is the bare word alone. Batch: `b869b-path-rest.md` (Opus, `Fixes #869`; converter R48 also writes `if let`), `e12-2.md` (Opus; uses `check_visit`, not `emit_walk`, so R-6 is not needed). Then: E12-3, #868 move, the guide's Memory workaround undone (ff8d00d, #872 fixed), #851 `decision`.
**2026-10-27 (architect after 464ff6d, batch done):** #655 closed by its last step (db99e12, `Fixes #655`; Changelog lines in b655-expr-view-in-tools and b655-two-ref-lifetime reports, version raise owed). C7-r1 done (c812c54, c3a526b, 4e27ccf; 78.0 % Nikaia). CI on 4e27ccf..464ff6d was running. Batch: `b871-int-literal-pair.md` (Sonnet, `Fixes #871`), `e12-1.md` (Opus), `b869a-path-narrowing.md` (Opus, #869 steps 1/2/4). Then: b869b (#869 steps 3/5/6, `Fixes #869`), E12-2, #868 move, #851 `decision`.
**2026-10-27 (architect after b9aa489, cut at b655-null-arm-view on `partial`):** #655 step 2a done (6c24a30), 2b built (da5a86e, null arm; Changelog lines in both reports, version raise owed); `check_arguments_at` stays: in tools files `ref Expr = given[i]` is NK1257 (Expr/Ty taken as words; cause not found, ADR-354 D1 decides it, no owner question). C-7 rest table done (11e85bf). Step 5 reproduces (two `ref` params, rustc missing lifetime, also for `-> ref E`). Batch: `b655-expr-view-in-tools.md`, `c7-r1.md` (Opus), `b655-two-ref-lifetime.md` (`Fixes #655`). Then: E12-1, #868, #851 `decision`.
**2026-10-26 (architect after fe7168d, cut at b655-push-local-view on a Deviation the brief allowed, a false cut):** #655 step 1 done (2e1ba5d, `Part`; Changelog line in its report). `check_arguments_at` kept for the `null`-arm case (item 1 *if into a view*). Rule in common.part: a choice the brief left to the builder is no deviation. Probes on 2e1ba5d: `Vec[ref E?]` push and `(i64, ref E?)` tuple part fail (E0308/NK1103), `if c { null } else { given[1] }` NK1103, `if c { t } else { given[1] }` and `y = t` now work. Batch: `c7-rest-plan.md`, `b655-element-view-slots.md` (2a), `b655-null-arm-view.md` (2b, drops `check_arguments_at`). Then: first C-7 rest slice from the table, #655 step 5, #868 move, #851 `decision`.
**2026-10-26 (architect after 602904c, batch done):** #655 step 4 done (4b deed9ad, 4a redo f4dfb8c; Changelog lines in both reports, version raise owed); CI green to deed9ad, f4dfb8c running. #655 items 1, 2, 5 still reproduce (probes with a struct `E`: `ref i64?`/`ref String?` are now NK1257/NK1261). #868: the dead key was fixed in 356a2e3; only the move of `lowered.sh` to Nikaia is left (low). Batch: `b655-push-local-view.md` (step 1), `c7-rest-plan.md`. Then: first C-7 rest slice from the table, #655 steps 2 and 5, #868 move, #851 `decision`. `docs/briefs/RED` is an empty untracked leftover.
**2026-10-26 (architect after 7280a0b, RED arm64/bootstrap_compiler):** 9936c70 (#655 4a, a statement begins a line) broke `it` tests holding one-line programs (its brief ran `-- parse` only); rule in builder-facts: a parser change runs the whole `it` binary. Its report's Changelog set to none. Uncommitted b655-operator-types files in the checkout are an interrupted run of that brief (the brief now says so). Batch: `b-revert-9936c70.md`, `b655-operator-types.md`, `b655-one-statement-a-line-redo.md` (blocked if >80 tests fail: then split). Then: `c7-rest-plan.md` (was dropped from this batch), #655 steps 1/2/5, #868, #851 `decision`.
**2026-10-26 (architect after 252bcf2, batch done):** fix-forward e474300, #655 step 3 (2003b4c, `Part`), #867 reader (0e24e46, `Fixes #867`) done; Changelog lines in b655's and b867-seeds-reader's reports (version raise owed); CI on 2003b4c/0e24e46 was running. #655 step 4 split: 4a parser (a statement begins a line, Part I 3.1 text in the brief), 4b checker (NK1191 widened to text/bool, unary `-`/`!`; `!5` stays valid, ADR-285 D3). Batch: `b655-one-statement-a-line.md`, `b655-operator-types.md`, `c7-rest-plan.md` (Opus, docs-only slice table for check/mod.rs and emit/). Then: brief the first C-7 rest slice from that table, #655 steps 1/2/5 (and `1 < "x"`), #868, #851 `decision`.
**2026-10-26 (architect after 75eede3, cut at b867-seeds-tables on Found):** b867 tables done (6cd8fea, `Part #867`). develop red since 7970db8 (`drive_actions.stdout` lacks drive.nika's tidy line): fix-forward first. Rule in builder-facts: a script change runs the tests naming it. Batch: `b-fixforward-drive-tidy.md`, `b655-mut-param-coalesce.md`, `b867-seeds-reader.md` (Opus, `Fixes #867`). Then: #868, #655 steps 1/2/4/5, C-7 rest research brief, #851 `decision`.
**2026-10-26 (architect after fbc9bfd, cut at b645 on a verification note under `Deviation`, a false cut):** #645 done (3ec1f1e, 1702057 `Fixes`; Changelog line in b645's report, version raise owed). Rule: how a test was verified goes under `Tests:`, not `Deviation:` (common.part). Filed #868 (`ready`, `benches/overlap/lowered.sh` dead `ordering` key, move to Nikaia). #655 item 2 reproduces (E0507). Batch: `b867-seeds-tables.md`, `b655-mut-param-coalesce.md`. Then: #867 reader walks, #655 steps 1/2/4/5, #868, C-7 rest research brief, #851 `decision`.
**2026-10-26 (architect after 41f30ed, cut at b867 on a planned `partial`):** #665 done (bcdbeca), #867 writer half done (38697bf, `Part`); both Changelog lines in their reports. #689 closed by hand (refused at the push with `.clone()` help, as Part I's lending `for` says). #645: items 2-3 no longer reproduce; item 1 specified this round (Part I *The branches meet on nothing*: a branch that changes a binding the other names, `NK2104`; supervisor owes the version raise with the build). Batch: `b645-overlap-shared-mut.md`, `b867-seeds-tables.md`. Then: #867 reader walks (`read_with`, `read_barriers`, 94-495), open bugs #655, C-7 rest research brief, #851 `decision`.
**2026-10-26 (architect after f691147, batch done):** #866 (662fb73) and #706 (998b622, tests only; did not reproduce) done, close on promote; CI on them in progress. #665 still reproduces. Filed #867 (`ready`, seeds.rs move in brief form). Batch: `b665-lent-let-returned.md` (bugs), `b867-seeds-writer.md` (self-hosting). Then: C-7 rest research brief (`check/mod.rs`), open bugs #689 #655 #645, #851 `decision`.
**2026-10-26 (architect after 5aebafd, cut at b-fixforward-script-tests on an informational Found, a false cut):** fix-forward 1aa6a97 pushed, its CI run in progress (if red: fix-forward again). #706 no longer reproduces (probes in the brief): test-only brief pins it. Batch: `b866-inferred-number-local.md`, `b706-views-into-param-vec.md`. Rule: `Found:` only for a defect outside the change (common.part). Then: the self-hosting lane (C-7 rest research brief, `emit/seeds.rs`), #851 `decision`.
**2026-10-26 (architect after 971b5ac, cut at b863 on a noop's Deviation note, a false cut):** #863 closed by hand (not reproducible). develop red since 6370390 (docs-only pushes run everything): four script tests (`agent_script`, `batch_frontmatter`, `batch_lines`, `drive_actions`) hard-code `target/fast/nikaia`, which CI lacks. Fix-forward `b-fixforward-script-tests.md`, then `b866-inferred-number-local.md`. Rules: a noop's reason goes under `Tests:` (common.part); *CI has no target/fast* in builder-facts. CI job logs (blob storage) are blocked by the proxy: use `check-runs/<job>/annotations`. Then: the self-hosting lane (C-7 rest research brief, `emit/seeds.rs`), #851 `decision`.
**2026-10-26 (architect after 79a95b3, cut at b865 on Found):** #865 done (635b59f, closes on promote; Changelog line in its report). Found filed as #866 (`ready`: `.to_string()` of an inferred number local / tuple field, `buffers.nika`). Batch: `b863-exit-code.md`, `b866-inferred-number-local.md`. Then: the self-hosting lane (C-7 rest research brief, `emit/seeds.rs`), #851 `decision`.
**2026-10-26 (architect after d33d355, cut at b864 on Found):** #864 done (6ba5c74, closes on promote; Changelog line in its report). Found filed as #865 (`ready`: `let s = t.len().to_string(); return s.trim()` fails in rustc; `formats` in `buffers.nika`). Batch: `b865-to-string-local-view.md`, `b863-exit-code.md`. Then: the self-hosting lane (C-7 rest research brief, `emit/seeds.rs`), #851 `decision`.
**2026-10-26 (architect after 1db1a4e, cut at b848 on `Deviation: none (note)`, a false cut):** #848 done (39ebe78, closes on promote). Rule fixed in `common.part` and the report form: bare `none`. Batch: `b864-plus-local-view.md`, `b863-exit-code.md`. Then: the self-hosting lane (C-7 rest research brief, `emit/seeds.rs`), #851 `decision`.

**2026-10-26 (architect after 8335430, drive.nika cut at b-drive-loop on Found):** #861, #862, drive.nika done (close on promote). Found filed as #864 (`+` local returned as a view lowers to `&'static str`; matrix in the issue). Batch: `b848-map-get-tuple-text.md`, `b864-plus-local-view.md`, `b863-exit-code.md` (#863 did not reproduce in two probes: brief allows `noop`). Then: the self-hosting lane (C-7 rest research brief, `emit/seeds.rs`), #851 `decision`.

**2026-10-26 (restart, develop c6a6bdd3):** develop red on befae38b/830d1970 (`nikaia_test::the_compiler_test_package_passes_at_both_settings`), fix-forward `docs/briefs/b-fixforward-compiler-tests.md` running. `main` sits on c6a6bdd3, untested (#861 fired twice). Next batch, in order: `b861-ci-docs-only-base.md`, `b848-map-get-tuple-text.md`, `b-batch-status-lines.md` (#862), all Sonnet. Then: #863 (process::exit code, reduce first), the self-hosting lane (C-7 rest `check/mod.rs` research brief, `emit/seeds.rs`; `priorities.md` table), #851 is `decision`. Process fixes of this restart: briefs in `docs/briefs/`, `common.part`, `Gate:` in the report form, fresh-container build.

Older entries: 2026-10-24, develop 8d56f29c, 0.0.1204.
Branch `wip/bounding-candidates-2026-10-19` (stash on 0.0.1080) checked
2026-10-24: everything in it is on develop in a later form (`bounding_grew`
returns the failed counters, `bounding_for`/`bounding_while` re-walk in a
loop); no order owed, branch deletable.
Update 2026-10-24 later: U-d4 done (steps 1-3, dfb42b5f); #667 per-file cache key step running; stage-2 worktree plan (#125 comments) awaits the owner; #799, #800 filed (`ready`).
Update after daf562f3: U-d6 landed in part; what is left of C-7 and E10/E11 is the table at the end of `selfhosting-plan.md`. C7-a..c and E10-0 have no prerequisite; C7-d waits for #804, E10-1 for #791. #801 is built (0.0.1212); #803 and #804 are `ready`.
Tests in Nikaia (owner, 2026-10-24): #821 `ready`; ADR-269 D22 (`tests/NAME.nika` + `.stdout`/`.stderr`) is specified but unbuilt (probe: `output_tests` in `project.rs` sees only `.stdout`/`.out`); step 1 builds it, then new program tests go there.
Update 2026-10-25 (fresh architect): #837 running, then briefs 838, 744, 582, 747 (scratchpad `briefs/`); then E10-4 adapter switch (`priorities.md` table). #825 closed by hand.
Update 2026-10-25 end of architect run (~65 turns): done #753 #780 #838 #744 #839 #582 #840, #747 step 1; #778 #775 #825 closed by hand. Next: briefs `e10-4-switch.md` (Opus), then `native-compiler.md` (alone; then evaluate its CI CPU log after ~20 runs for the owner's question on runner CPUs). Open small bugs: #841, #747 step 2. Owner direction: optional native compiler build, `native` always resolved to the CPU name in flags and cache keys.
Update 2026-10-25 (architect after 087ce275): briefs `841.md`, `747-2.md` (Sonnet), `e11-1.md` (Opus), serial. Native tool partial: count owed with a non-AVX-512 CPU name (x86-64-v3). d995ae5a red in bootstrap_compiler only, fixed by later green runs.
Update 2026-10-25 later: done #756 #842 E11-1..3, release script; #843 reopened `blocked` (contradicted ADR-282 D8, revert brief `fix-843-revert.md` running, develop red until it lands). Next: E11-4 (the core, ~3600 lines) needs a brief; automation step `scripts/batch.nika` (brief `batch-script.md`).
Update 2026-10-25 (architect after d9d3f921): #843 restated `ready` (tier walk vs D8; no owner decision). Batch `843-tiers.md` (Sonnet), `cycle-script.md` (Sonnet), `e11-4a.md` (Opus). E11-4 split 4a/4b/4c (`selfhosting-plan.md`). Owner (merge duties): main-session cycle above; reminders go to `DIR/owner.txt` (architect writes); `Found` lines become issues by the architect, not the main session.
Stage: 1 (serial). Current issue: #125, C-7 of `selfhosting-plan.md`: U-d0 and
U-d1 done, **U-d2** (`check/attributes.rs`, 714 lines, to
`tools/check_attributes.nika`) running; then **#667** (owner: before U-d3):
steps 1-3 are in `sysroot.rs` (NIKAIA_LOWER_TIMES, `LoweredBefore`,
`in_parallel`), but the key is the whole package plus the binary's hash, so the
real cycle (edit one .nika, rebuild) always misses; order is per-file keys on
file + package interface, then step 4. Then U-d3 .. U-d6, then E10/E11.
Modular compiler (owner's question): basis in `modular-compiler.md`, #793
(`decision`); #667 step 4 is the next order after the current #667 builder.
Long jobs (owner's question): basis in `long-jobs.md`, #794 (`decision`); no measuring agent; CI is the verifier, the main
session watches it. develop red since 9f090392 (badges check, arm64 Test):
the repair comes before the next step.
CI caches: #795 parked by the owner (plan insufficient; more cacheability,
#667 step 4, first); no order.
Formal verification, #796 (`decision` while question 3 is open): owner chose
claim (b), the compiler's Nikaia code free of run-time errors, riding with
stage 1; "formally verified" stays off README/website/guide (rule in
`formal-verification.md` 6). First order owed: `scripts/proved.nika` (counts
over `tools/*.nika`), slotted after the running builder. Formal semantics:
basis in `formal-semantics.md`, owner to decide. #386 and #388 lack a status
label (noted to the main session).
Facts about data (owner, 2026-10-24): ADR-381 decided (a pure `bool` call is a
named property, proved by name, carried, checked once); #797 `ready` (step 0:
two defects, `run --unproved-claims` ignored on a file and a struct rule over
an inline list literal; then steps 1-3 in `prove_terms.nika`); #798
`decision` (Q1 element rules `xs.all(..)`, Q2 std `is_sorted`/`binary_search`);
basis `facts-about-data.md`. No interpreter needed for claim (b)
(`formal-verification.md` 7). Supervisor owes the version raise for ADR-381.
Known risk for new tools files: #791 (methods matched by bare name).
Parked: branch `wip/bounding-candidates-2026-10-19` (loop counter candidates in
the bounds analysis, ~24 lines, 120 versions stale): not merged, no order until
self-hosting reaches `bounds`; re-done fresh then, the branch is a sketch only.
