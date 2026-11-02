Result: done
Cause: b016c7c8 (S2 bounds report) - bootstrap_compiler: `badges.py --check` red, self-hosted.svg out of date (bounds_report.nika had no row in scripts/self_hosting.py)
Fixed: added the bounds_report.nika row and regenerated assets/badges/self-hosted.svg, both computed on a clean HEAD export
Tests: badges.py --check on a HEAD export + the fix: green; pre-push.nika (staged): index, fmt, adr-refs, badges green, version red (supervisor's, not touched). Working-tree `badges.py --check` is red only because other agents' uncommitted edits shift the share
Found: none. Note: the shared tree holds another agent's uncommitted `diagnostics.nika` row in self_hosting.py (file untracked); I committed only my hunk via `git apply --cached`, so `scripts/commit.nika` was not used (it adds the whole file). That row needs its own badge refresh when diagnostics.nika is committed
Files: scripts/self_hosting.py assets/badges/self-hosted.svg
Commit: self_hosting: bounds_report.nika's row and the badge (9fdba933)
