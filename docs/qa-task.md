# QA: the suite is red

The driver ran the complete suite after the batch's builders committed, and it
is red. You are the QA agent (`.claude/agents/qa.md`): find which unpushed
commit broke what, fix it forward as its own commits, and report. This file is
the template (`scripts/drive.nika`, `drive_qa_copy`); the driver runs QA on the
lane's copy, with the two paths below made the lane's own.

* **Input**: `docs/briefs/qa-input.txt`. It holds the red steps, the failed
  jobs' log tails (`suite.log`), the unpushed commits and the briefs and
  reports behind them. Read the failing lines first, then only the brief and
  report of the commit that touched the failing area.
* `gate-error` with no step lines means the driver could not read the CI
  result: look at the CI run on the lane's `ci/<lane>` branch
  (`gh api repos/Nikaia-Foundation/Nikaia/actions/runs?branch=ci/<lane>`) and
  its failed job's log before anything else.
* Fix forward, never skip or loosen a test; cheap checks only (the failing
  tests, the tests your fix touches, `cargo fmt`, clippy when Rust changed,
  `nikaia run scripts/pre-push.nika`). The driver runs the suite again after you.
* Commit locally with `target/fast/nikaia run scripts/commit.nika -- REPORT`
  (it adds exactly the report's `Files:`); no push. One commit per cause.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's.
* No cause after about 60 turns: commit nothing half-made, `Result: partial`
  with what you ruled out, the smallest probe and the suspected file and
  function.

## Report (at most 15 lines, to exactly `docs/qa-task.md.report.md`)
Fields: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Issue, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* Fields with nothing to say hold the bare word: `done`, `none`.
* `Result: done` only when every red step is fixed; `partial` otherwise.
* `Found:` only for a defect this run cannot fix (the driver files it as an issue).
* `Issue: complete`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...`).
