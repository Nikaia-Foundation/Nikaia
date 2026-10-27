Result: done
Files: scripts/commit.nika docs/architect.md docs/briefs/common.part tests/compiler/tests/commit_dry_run.nika tests/compiler/tests/commit_dry_run.stdout
Commit: commit.nika: brief of a .md.report.md, Body: field
Body: The brief of `X.md.report.md` is `X.md` itself (the old `X.md.md` was never found, so the Attribution block was lost); `<brief>.report.md` still reads `<brief>.md`. A report's optional `Body:` line is a paragraph after the subject.
Pushed: a6d3bd869120c9080b86d716c4aa708ed7cf1782
Tests: commit_dry_run + whole tests/compiler at --both-settings (173) - pass; guard set pass; new test's script run by hand against the old commit.nika: no Attribution, no Body
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
