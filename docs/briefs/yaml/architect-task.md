---
agent: architect
model: opus
max_turns: 80
budget_usd: 25
---
# Task: decide the next `yaml` batch (started by `scripts/drive.nika -- docs/briefs/yaml`)

This lane builds the official package `yaml` (#928) by ADR-yaml-1 (`docs/specification/adr/adr-yaml-1.md`). Another session's driver runs `docs/briefs/` with its own `docs/briefs/batch.txt`: never touch that file nor any brief outside `docs/briefs/yaml/`.

Read `docs/architect.md`, `docs/briefs/yaml/architect-input.txt` (the stop line the driver stopped on, for `RED` also the failing jobs), the reports of the last batch (`docs/briefs/yaml/*.report.md`), ADR-yaml-1 and #928's *To build*. Then:

1. Decide the next batch of 2-3 briefs (not more) in `docs/briefs/yaml/`, in #928's *To build* order, and rewrite `docs/briefs/yaml/batch.txt` with their file names in order. Each brief has the frontmatter (`agent: builder`, `model: opus`, `max_turns`, `budget_usd`), the rules block of `docs/briefs/common.part` with the package gate (copy it from `y1-tree-core.md`), and its report path `<full brief path>.report.md`. Size each for one run (about 150 minutes).
2. A `Found:` or a `blocked` report on a compiler, lowering or `std` gap: file it (brief form, label `ready`, or `decision` with a decision paper where it is not decided), and make its fix a brief of this batch, or wait for it; never a brief that writes around it (CLAUDE.md, *A missing form stops the work*).
3. A red CI: fix it forward only when a commit of this lane (`packages/yaml/`, `docs/briefs/yaml/`, a fix brief of this lane) broke it; otherwise leave `batch.txt` as it is and report `Result: blocked`, `Owner: none`, `Batch: unchanged (red not from yaml: <jobs>)`: the main session's lane fixes it.
4. Update #928's *To build* (keep its decisions) and the label of every issue you change (exactly one status label; #928 keeps `ready` + `wip`).
5. Commit and push your own files (`git add <files>`, `git pull --rebase origin develop`, `git push origin develop`).
6. GitHub only through `gh api repos/nikaia-foundation/nikaia/...` (REST; GraphQL and the MCP tools are not available headless).

Report to exactly `docs/briefs/yaml/architect-task.md.report.md` (the driver deletes the old one before the run) with:

* `Result: done|partial|blocked` (the bare word first)
* `Batch:` the briefs in order
* `Owner:` a question for the owner, or `none`

The architect does not count as a batch brief.
