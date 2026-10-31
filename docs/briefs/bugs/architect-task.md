---
agent: architect
model: opus
max_turns: 80
budget_usd: 25
---
# Task: decide the next bug batch (started by `scripts/drive.nika -- docs/briefs/bugs --no-bump`)

This lane fixes open compiler, lowering, `std` and toolchain **defects** (kind 3 of CLAUDE.md, *A missing form stops the work*: specified or plainly meant, and not working). It runs in the same session and checkout as the main lane, whose driver runs `docs/briefs/` with its own `docs/briefs/batch.txt` and moves Rust into Nikaia: never touch that file nor any brief outside `docs/briefs/bugs/`. The main lane may append to this lane's `batch.txt` a brief for a defect a move found (with a `Blocks:` line). The main lane's architect skips issues labelled `wip`; this lane claims an issue by setting `wip` when it writes its brief and takes `wip` off when the issue is closed or handed back.

Read `docs/architect.md`, `docs/priorities.md`, `docs/briefs/bugs/architect-input.txt` (the stop line the driver stopped on, for `RED` also the failing jobs), and the reports of the last batch (`docs/briefs/bugs/*.report.md`). Then:

1. List the open issues (`gh api repos/nikaia-foundation/nikaia/issues?state=open&per_page=100`, all pages) and pick the defects labelled `ready` that no other lane holds (no `wip`, not named in `docs/briefs/batch.txt` or `docs/briefs/yaml/batch.txt`). A brief in `docs/briefs/bugs/` with a `Blocks: <move brief>` line (the main lane put it there for a defect a move found) stays in `batch.txt` and goes first, before the briefs below. Order the rest by `docs/priorities.md`'s score: a defect a builder is blocked on first, then one that gives a wrong value or a wrong refusal, then the rest; cheap and certain before large. Skip feature requests, decisions, notes, self-hosting moves and converter work: they are not this lane's.
2. Decide the next batch of 2-3 briefs (not more) in `docs/briefs/bugs/`, and rewrite `docs/briefs/bugs/batch.txt` with their file names in order. Name each brief `b<N>-<issue>-<topic>.md`. Each brief has the frontmatter (`agent: builder`, `model: opus`, `max_turns`, `budget_usd`), the rules block of `docs/briefs/common.part`, the issue's probe, the test it ends with (a `.nika` in `tests/compiler/` with its `.stdout`/`.stderr`, CLAUDE.md: tests in Nikaia only), `Fixes #N` for the commit that finishes the issue, and its report path `<full brief path>.report.md`. Size each for one run (about 150 minutes); split a large defect into steps on the issue's *To build* list.
3. Set `wip` on every issue a brief of this batch fixes (it keeps its status label `ready`).
4. A new `NK` code is taken only from this lane's block: `NK1312`-`NK1319` and `NK2708`-`NK2719`. A defect that needs a code in another range: the brief says so and the builder reports it; the supervisor reserves it. This lane never raises the version, never writes `CHANGELOG.md` nor the badges: the main driver does it (`--no-bump`); a builder names in its report what its change decided.
5. A `Found:` or a `blocked` report on a gap that is not a defect (kind 2): file it with label `decision` and a decision paper, take `wip` off the issue it blocks, and go on with the next defect.
6. A red CI: fix it forward only when a commit of this lane (`docs/briefs/bugs/`, a fix brief of this lane) broke it; otherwise leave `batch.txt` as it is and report `Result: blocked`, `Owner: none`, `Batch: unchanged (red not from bugs: <jobs>)`: the main lane fixes it.
7. Keep the labels current: exactly one status label per issue, plus `wip` while this lane holds it.
8. Commit and push your own files (`git add <files>`, `git push origin develop`; a rejected push: `git pull --rebase origin develop` only when the tree has no unstaged changes of yours, else report it).
9. GitHub only through `gh api repos/nikaia-foundation/nikaia/...` (REST; GraphQL, `search/` and the MCP tools are not available headless).

Report to exactly `docs/briefs/bugs/architect-task.md.report.md` (the driver deletes the old one before the run) with:

* `Result: done|partial|blocked` (the bare word first)
* `Batch:` the briefs in order
* `Owner:` a question for the owner, or `none`

The architect does not count as a batch brief.
