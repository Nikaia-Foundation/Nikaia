Result: done
Files: scripts/ci-changes.sh
Commit: CI: a docs-only push runs everything while main lacks a code change below it
Fixes: #861
Pushed: 63703908eddfc56995050931a0f983ea1fb3e7ee
Tests: probe in scratch clone: origin/main at parent of code+docs commits -> docs=false; origin/main at code commit -> docs=true (EVENT=push REF=refs/heads/develop BASE=<code commit> GITHUB_OUTPUT=/dev/stdout sh scripts/ci-changes.sh)
Gate: green (shell script only; no compiler or Rust change)
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
