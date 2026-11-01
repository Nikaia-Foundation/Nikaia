---
agent: architect
model: opus
max_turns: 80
budget_usd: 25
effort: medium
---
# Task: decide the next `tools` batch (started by `scripts/drive.nika -- docs/briefs/tools`)

This lane builds `json::read` through grammar rules (#1019, ADR-grammar-2, `docs/specification/adr/adr-grammar-2.md`; Part II 10.8). Other sessions' drivers run `docs/briefs/` (self-hosting), `docs/briefs/bugs/` and `docs/briefs/yaml/`: never touch their `batch.txt` nor any brief outside `docs/briefs/tools/`.

Read `docs/architect.md` lines 1-260 (roles, briefs, batches; its *State* section is the main lane's), `docs/briefs/tools/architect-input.txt` (the stop line the driver stopped on, for `RED` also the failing jobs), the reports of the last batch (`docs/briefs/tools/*.report.md`), ADR-grammar-2 and #1019's *To build*. Then:

1. Decide the next batch of 2-3 briefs (not more) in `docs/briefs/tools/`, in #1019's *To build* order, and rewrite `docs/briefs/tools/batch.txt` with their file names in order. Name them `j<N>-1019-<topic>.md` (j1, j2 are done). Each brief has the frontmatter (`agent: builder`, `model: opus`, `max_turns`, `budget_usd`), the sections of `j2-1019-match-on-a-kind.md` (Score, Problem, Decision, Locations, To build, Done when, Stop and report if, Not yours), and its rules and report block copied from `## Rules` to the end of `j2-1019-match-on-a-kind.md`, with the report path replaced by `<full brief path>.report.md`. Size each for one run (about 150 minutes): step 3 of #1019 (`members[S]`, D3-D5) is likely two briefs (the build of `members[S]` with its mapping rule and `NK1311`/`NK1312`; then the path on the `ParseError` stack and unescaped strings as views).
2. A `Found:` or a `blocked` report on a compiler, lowering or `std` gap: file it (brief form, label `ready`, or `decision` with a decision paper where it is not decided), and make its fix a brief of this batch, or wait for it; never a brief that writes around it (CLAUDE.md, *A missing form stops the work*). A `Needs: architect decision`: decide it by `docs/architect.md`, *Language decisions*.
3. A red CI: fix it forward only when a commit of this lane broke it; otherwise leave `batch.txt` as it is and report `Result: blocked`, `Owner: none`, `Batch: unchanged (red not from tools: <jobs>)`.
4. Update #1019's *To build* (keep its decisions; tick what the reports built) and the label of every issue you change (exactly one status label; #1019 keeps `ready` + `wip`).
5. Commit and push your own files (`git add <files>`, `git push origin develop` fast-forward). No `git pull --rebase`, `--autostash` or `git stash` in the shared checkout; a rejected push is reported (`Pushed: rejected`), the driver rebases.
6. GitHub only through `gh api repos/nikaia-foundation/nikaia/...` (REST; GraphQL and the MCP tools are not available headless).

## Lane facts

* **No new Rust** (owner, 2026-10-31, CLAUDE.md): every brief builds in Nikaia (`.nika`); a step that needs hand-written Rust is not briefed: file the Nikaia gap that forces it and brief that gap first.
* `NK` codes: this lane uses those ADR-grammar-2 names (`NK1250`, `NK1310`, `NK1311`, `NK1312`, already in Part II/III). Any new code: `NK1320`-`NK1329` only.
* An empty `RED` file in this directory is not a red CI: `architect-input.txt` decides.
* Re-queue a blocked brief: rename its report to `<brief>.runN.report.md` (`git mv` when tracked), and add a paragraph to the brief on what changed between the runs.
* Known open: #1079 (`f[i64?](x)` does not parse in a call's brackets) is for the bug lane; a brief here that needs it waits.
* Version raises are the supervisor's: no brief touches version lines, `CHANGELOG.md` or badges.
* winnow-grammar (the backend) is the owner's repo `https://github.com/keywan-ghadami/winnow-grammar`: a builder changes it in its own clone `/home/user/winnow-grammar`, saves the commit with `format-patch` into this directory and reports `Pushed: winnow-grammar <sha>`; the main session pushes it, and the pin in `Cargo.toml` moves in a later brief. Never a `[patch]` in the shared checkout; no `gh api` on that repo (403).

Report to exactly `docs/briefs/tools/architect-task.md.report.md` (the driver deletes the old one before the run) with:

* `Result: done|partial|blocked` (the bare word first)
* `Batch:` the briefs in order
* `Owner:` a question for the owner, or `none`

The architect does not count as a batch brief.
