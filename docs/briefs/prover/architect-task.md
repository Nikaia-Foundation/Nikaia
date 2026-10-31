---
agent: architect
model: opus
max_turns: 80
budget_usd: 25
---
# Task: decide the next `prover` batch (started by `scripts/drive.nika -- docs/briefs/prover`)

This lane completes the prover (owner, 2026-10-31, below). Other sessions' drivers run `docs/briefs/` (self-hosting moves), `docs/briefs/bugs/` and `docs/briefs/yaml/`: never touch their `batch.txt` nor any brief outside `docs/briefs/prover/`.

## The owner's order (2026-10-31, verbatim, German dictation)

> Der Beweiser soll vervollständigt werden. Wir bekommen immer wieder Hinweise darauf, dass die Sprache ja nur bestimmte Sachen beweisen kann, aber eben nicht zum Beispiel die Länge von Listen oder dass eine Liste sortiert ist und. Auch, dass der Beweiser noch kein eigenständiges, vollständiges Produkt wäre. Du sollst jetzt mit einem Builder-Agent und einem Architekten zusammen und dem Treiber, also dem Skript, diese Lücken nach und nach schließen. Priorisiert werden soll dabei, was das Vertrauen in den Compiler erhöht, das heißt auch das Einsetzen von Asserts in den Code. Insbesondere aber, es gibt einen Bericht im Repository, der sagt, dass ein gewisser Prozentsatz derzeit bewiesen wäre im Compiler, was ich glaube, bestimmte Zugriffe angeht. Da müssen wir natürlich 100% erreichen und nicht irgendwie einen Bruchteil. Ich hoffe, du findest den Bericht, den ich da im Hinterkopf habe, und kannst daraus schließen, was die Zielsetzung ist. Also wichtig ist die Priorisierung auf alles, was die Außenwirkung vom Compiler in Bezug auf Zuverlässigkeit verbessert. Und Nebenziele sind den Beweiser als eigenes Produkt und den Beweiser weiter zu vervollständigen, auch mit so Schnittstellen an LLMs und ähnliches, um Beweise besser finden zu können, oder auch den Einsatz neuester Forschung. Aber eins nach dem anderen, und deshalb soll der Architekt ja auch priorisieren.

The report the owner means is `docs/formal-verification.md` (#796): "1,473 indexes, 802 proved (54 %); 1,101 arithmetic steps, 491 proved (45 %)" in `tools/*.nika`, and claim (b), decided 2026-10-24: the compiler's Nikaia code proved free of run-time errors. The owner now sets the target: **100 %** of the indexes and arithmetic steps of the compiler's Nikaia code proved, not a share. The supervisor's reading of the order (correct it in your report if the records say otherwise):

1. **First, what the outside sees about the compiler's reliability**: the count of proved / unproved checks in `tools/*.nika`, measured per commit by a tool (`docs/formal-verification.md` section 5's `scripts/proved.nika`, if not built), published only as honest claims (section 4, `docs/writing-guide.md`); then the unproved checks brought to zero, file by file, the certificate checker first (item 5); `assert`s placed in the compiler's code where they state an invariant the prover can then use. Sort the unproved checks by the form that blocks them (an `if` value, a length, a counter, a fact about a table...) and take the prover extension that frees the most first: an extension every user also gets beats rewriting compiler code to suit the prover.
2. **Then the gaps outsiders name**: lengths of lists, a list that is sorted, facts about collections (`docs/facts-about-data.md`, #797, #798, #386, #432, #388), postconditions, lemmas.
3. **Side goals, later, by the score**: the prover as a product of its own (a command, documentation, a page that says what it proves and what not), interfaces for LLMs and other tools to find proofs (hints, certificates in and out), current research.

## What you do

Read `docs/architect.md` lines 1-260 (roles, briefs, batches; its *State* is the main lane's), `docs/priorities.md`, `docs/formal-verification.md`, `docs/facts-about-data.md`, `docs/briefs/prover/architect-input.txt` if it exists (the stop line), the reports of the last batch (`docs/briefs/prover/*.report.md`). Then:

1. **On the first run** (no `docs/briefs/prover/state.md`): file an epic issue for the 100 % target (brief form, label `ready`, a *To build* list in the order above, the current figure measured on today's binary; reuse #796 or an existing issue if one fits), and write `docs/briefs/prover/state.md`: the lane's goal, the figure, the order of work with scores, decisions in force. Every later run rewrites it (current state only).
2. Decide the next batch of 2-3 briefs (not more) in `docs/briefs/prover/`, named `p<N>-<issue>-<topic>.md`, and rewrite `docs/briefs/prover/batch.txt`. Each brief: the frontmatter (`agent: builder`, `model: opus`, `max_turns`, `budget_usd`), `Facts:`, *Locations* (`scripts/where.nika`), the steps with their tests (`.nika` in `tests/compiler/`, CLAUDE.md), the score lines, the rules block of `docs/briefs/common.part` with its report path `<full brief path>.report.md` filled in. Size each for one run (about 150 minutes). Probe every claim a brief makes on today's binary (`docs/architect.md`, *Before a brief is written*).
3. A brief that edits `tools/*.nika` (an `assert`, a guard) names its files and checks they are not in `docs/briefs/batch.txt`'s current briefs (the self-hosting lane moves code into them); a changed `.nika` re-lowers its file in `crates/nikaia-std/src/tools/lowered/` (`lower-std`, builder-facts).
4. A gap that is not decided (kind 2, CLAUDE.md *A missing form stops the work*), such as a new syntax for postconditions or quantifiers: a `decision` issue with a decision paper (where from, alternatives with doing nothing, for and against, other languages and tools such as Dafny, SPARK, Verus, Liquid Haskell, recommendation) and `Owner:` in your report; meanwhile go on with what needs no decision.
5. A defect found on the way is fixed in this lane when it is the prover's; otherwise filed (`ready`) for the bug lane.
6. New `NK` codes only from this lane's block: `NK1320`-`NK1329`, `NK2720`-`NK2729`. A changelog sentence goes into each builder report's `Changelog:` line; the driver raises the version.
7. A red CI: fix it forward only when a commit of this lane broke it; otherwise it does not stop this lane (its pushes are gated by the driver's local suite): plan the batch as usual and report `Result: done`, noting the red in `state.md`.
8. Keep labels current (exactly one status label; the issues this lane builds carry `wip`).
9. Commit and push your own files (`git add <files>`, `git push origin develop` fast-forward). No `git pull --rebase`, `--autostash` or `git stash` in the shared checkout; a rejected push is reported, the driver rebases.
10. GitHub only through `gh api repos/nikaia-foundation/nikaia/...` (REST).

Report to exactly `docs/briefs/prover/architect-task.md.report.md` (the driver deletes the old one before the run) with:

* `Result: done|partial|blocked` (the bare word first)
* `Batch:` the briefs in order
* `Figure:` proved / total indexes and arithmetic steps, today
* `Owner:` a question for the owner, or `none`

The architect does not count as a batch brief.

## Supervisor notes for the next run (2026-10-31, delete when handled)

* p4 (#1054 step 2) moved little: 729/1612 operations under `off`, 736 under `prove-through-loops:on`; it helps `while` only, and the compiler's code loops mostly with `for` (its range record is #432). Weigh #432 next, and whether `prove-through-loops` should be on by default for the count that is published.
* The owner asked for `assert`s in the compiler's code: the 379 "parameter no precondition bounds" sites (p3) are that work; plan it once the files are clear of the self-hosting lane.
* Defects found by the supervisor's script builders, not this lane's (file for the bug lane, `ready`, unless already filed): #1062 (filed: NK1104 on `s.to_string()`, workaround `exe.clone()` in `scripts/drive.nika`); `parts = f().split(" ").collect()` reassigned passes check, rustc refuses (temporary dropped while borrowed); a `Duration` in an f-string passes check, rustc refuses; `async e` (Part I 8.1.2) refused as a reserved word, `spawn fn {}` used in `scripts/gate.nika`; a bogus `Unused import: ProbePlain` warning with `spawn fn { f(s) }` where `f` throws and calls `process::run` (probe: `/tmp/claude-0/-home-user-Nikaia/e0e27fde-472b-560f-914b-b1002dd38405/scratchpad/gate-speed/spawn2.nika`, may be gone); NK1102 on a `fn limit() -> String { return "5G" }` result passed to an owning `String` parameter (Part I 2.2; probe `.../scratchpad/gate-speed/probe.nika`). Write a probe of a few lines into each issue; the scratchpad does not survive the session.
* p4's report wrote `Changelog: none` for a prover change; a prover change raises the version: give each builder brief a `Changelog:` reminder.
