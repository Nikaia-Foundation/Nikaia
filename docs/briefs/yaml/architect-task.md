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
5. Commit and push your own files (`git add <files>`, `git push origin develop` fast-forward). No `git pull --rebase`, `--autostash` or `git stash` in the shared checkout (CLAUDE.md: it holds other agents' files); a rejected push is reported (`Pushed: rejected` in the report), the driver rebases.
6. GitHub only through `gh api repos/nikaia-foundation/nikaia/...` (REST; GraphQL and the MCP tools are not available headless).

## Lane facts (so you need not look them up)

* Read only `docs/architect.md` lines 1-260 (roles, briefs, batches); its *State* section is the main lane's.
* An empty `RED` file in this directory is not a red CI: `architect-input.txt` decides (a `RED` line names jobs).
* Re-queue a blocked brief: rename its report to `<brief>.runN.report.md` (`git mv` when tracked) so `batch.nika` sees it unrun, and add a paragraph to the brief on what changed between the runs.
* The rules block of a brief: from `## Rules` to the end of the newest brief here, with `<full brief path>` (the report line) replaced.
* A case that reads a type of another package (`use yaml`): probe it as a two-file scratch package (`nikaia.toml`, `src/main.nika`, `tests/x.nika` + `.stdout`, `nikaia test --project .`); its test goes into `tests/compiler` (package `compiler_tests`, `use compiler_tests`), `docs/builder-facts.md`.
* `packages/yaml/tests` are not run by CI (only `tests/compiler` and the rs2nika projects are): the package gate in each brief is what guards them.

Report to exactly `docs/briefs/yaml/architect-task.md.report.md` (the driver deletes the old one before the run) with:

* `Result: done|partial|blocked` (the bare word first)
* `Batch:` the briefs in order
* `Owner:` a question for the owner, or `none`

The architect does not count as a batch brief.
