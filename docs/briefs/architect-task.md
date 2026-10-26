---
agent: architect
model: opus
max_turns: 80
budget_usd: 25
---
# Task: decide the next batch (started by `scripts/drive.nika`)

Read `docs/architect.md` and `docs/briefs/architect-input.txt` (the stop line the driver stopped on, for `RED` also the failing jobs). Then:

1. Decide the next batch of 2-3 briefs (not more) in `docs/briefs`, and rewrite `docs/briefs/batch.txt` with their file names in order. A red CI is fixed forward first.
2. Commit and push your own files (`git add <files>`, `git push origin develop`).
3. GitHub only through `gh api repos/nikaia-foundation/nikaia/...` (REST; GraphQL and the MCP tools are not available headless).

Report to exactly `docs/briefs/architect-task.md.report.md` (the driver deletes the old one before the run) with:

* `Result: done|partial|blocked` (the bare word first)
* `Batch:` the briefs in order
* `Owner:` a question for the owner, or `none`

The architect does not count as a batch brief.
