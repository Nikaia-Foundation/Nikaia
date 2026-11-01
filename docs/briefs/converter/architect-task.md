---
agent: architect
model: opus
max_turns: 80
budget_usd: 25
effort: medium
---
# Task: decide the converter's next block (started by `scripts/drive.nika -- --focus`)

Read `docs/architect.md` and `docs/briefs/converter/architect-input.txt` (the stop line the driver stopped on: `QUEUE EMPTY converter`, `CUT`, or `RED` with the failing jobs). Then:

1. Decide the next briefs of the converter area in `docs/briefs/converter` and rewrite `docs/briefs/converter/batch.txt` with their file names in order. On `QUEUE EMPTY converter` write at most 3 briefs. A brief that ran before and goes in again gets its old report renamed `BRIEF.run<N>.report.md` (the next free N), or `batch.nika` counts it as run. This area repairs the converter `rs2nika`: its sources `crates/rs2nika/convert/src/*.nika`, its tests `crates/rs2nika/convert/tests/`, its name ledger `crates/rs2nika/convert/rs2nika.names`; the kept share each move scored is in `docs/rs2nika-score.md` and in the moves' `Rs2nika:` lines (the focus log's `kept k/m`). Pick the patterns that cost the most kept lines first; a red CI is fixed forward first.
   Every brief carries `effort:` in its frontmatter: `low` where the pattern is known (a row in `rs2nika.names`, a form the converter already writes elsewhere, a test that names the expected draft), `medium` otherwise.
   A compiler defect a converter brief found: file it (`ready`, brief form), write its brief as `docs/briefs/bugs/b<N>-<issue>-<topic>.md` with a `Blocks: <converter brief>` line and append it to `docs/briefs/bugs/batch.txt`; the driver runs it before the next block. A `Needs: architect decision` gap: decide it by `docs/architect.md`, *Language decisions*.
   Skip issues labelled `wip` and other lanes' briefs; take no `NK` code from another lane's block.
2. Commit and push your own files (`git add <files>`, `git push origin develop`).
3. GitHub only through `gh api repos/nikaia-foundation/nikaia/...` (REST; GraphQL and the MCP tools are not available headless).

Report to exactly `docs/briefs/converter/architect-task.md.report.md` (the driver deletes the old one before the run) with:

* `Result: done|partial|blocked` (the bare word first)
* `Batch:` the briefs in order
* `Owner:` a question for the owner, or `none`

The architect does not count as a block brief.
