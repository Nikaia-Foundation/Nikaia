---
agent: architect
model: opus
max_turns: 80
budget_usd: 25
---
# Task: decide the next batch (started by `scripts/drive.nika`)

Read `docs/architect.md` and `docs/briefs/architect-input.txt` (the stop line the driver stopped on, for `RED` also the failing jobs). Then:

1. Decide the next batch of 2-3 briefs (not more) in `docs/briefs`, and rewrite `docs/briefs/batch.txt` with their file names in order. This lane holds self-hosting moves only, at least two a batch (`docs/architect.md`, *Lanes*); a red CI is fixed forward first.
   A compiler defect a move found: file it (`ready`, brief form), write its brief as `docs/briefs/bugs/b<N>-<issue>-<topic>.md` with a `Blocks: <move brief>` line, append it to `docs/briefs/bugs/batch.txt`, set `wip` on the issue; the move waits and re-enters this batch once the fix is on develop. Nothing is written around it.
   Every report since your last run (`git log --since` of the reports in `docs/briefs/` and `docs/briefs/bugs/`, and `qa-task.md.report.md`) with `Result: partial|blocked` and a `Found:` defect with a probe: file it the same way, unless an open issue holds it (then comment the probe there). A `Needs: architect decision` gap: decide it by `docs/architect.md`, *Language decisions* (obvious: ADR text and a `ready` build; not obvious: a `decision` issue and the `Owner:` line).
   Skip issues labelled `wip`: another lane holds them (the bug lane, `docs/briefs/bugs/`, fixes `ready` defects; the `yaml` lane, `docs/briefs/yaml/`, builds #928). Never touch their briefs, nor their `batch.txt` beyond appending such a brief. The bug lane's `NK` block is `NK1312`-`NK1319` and `NK2708`-`NK2719`: take no code from it.
2. Commit and push your own files (`git add <files>`, `git push origin develop`).
3. GitHub only through `gh api repos/nikaia-foundation/nikaia/...` (REST; GraphQL and the MCP tools are not available headless).

Report to exactly `docs/briefs/architect-task.md.report.md` (the driver deletes the old one before the run) with:

* `Result: done|partial|blocked` (the bare word first)
* `Batch:` the briefs in order
* `Owner:` a question for the owner, or `none`

The architect does not count as a batch brief.
