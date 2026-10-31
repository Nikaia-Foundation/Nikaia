---
agent: qa
model: sonnet
max_turns: 80
budget_usd: 15
---
# Task: fix forward a red suite (started by `scripts/drive.nika`)

The driver ran the complete suite (`scripts/gate.nika -- --all`) on the builders'
local commits and it is red. Read `.claude/agents/qa.md`, then the driver's
`qa-input.txt` (in the directory the driver runs on: `docs/briefs/qa-input.txt`
for the main lane, `docs/briefs/bugs/qa-input.txt` for the bug lane; the
driver's full log is `suite.log` beside it).

1. Find which unpushed commit broke which step, from the failing lines and
   that commit's brief and report.
2. Fix it forward, as its own commit(s), locally (`scripts/commit.nika --
   REPORT`, no push). Rerun the failing tests and the ones your fix touches.
3. Never skip or disable a test, never loosen an expected output.

Report to exactly `docs/briefs/qa-task.md.report.md` with:

* `Result: done|partial|blocked` (the bare word first)
* `Cause:` the commit (sha8) and the failing step, one line per cause
* `Fixed:` what changed, one line
* `Tests:` the tests rerun and their result
* `Found:` a defect you could not fix here, with a probe, or `none`
* `Files:` and `Commit:` as a builder's report
