Result: done
Cause: 65c3296b..ba6b114c (the unpushed lane; the self-hosted share moved) failed bootstrap_compiler at `scripts/badges.py --check`: self-hosted.svg out of date
Fixed: regenerated assets/badges/self-hosted.svg with scripts/badges.py (86.7 % -> 86.8 %); no code changed
Tests: `python3 scripts/badges.py --check` ok; other CI steps before it (fmt, ADR refs) were green in the log; later steps not run here (the driver runs the suite)
Found: none
Files: assets/badges/self-hosted.svg docs/briefs/qa/qa-task.md.report.md
Commit: The self-hosted badge is regenerated for the new share of Nikaia lines
Changelog: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
