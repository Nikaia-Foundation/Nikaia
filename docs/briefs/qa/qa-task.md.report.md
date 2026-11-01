Result: done
Cause: 547e6462 (self-hosted badge said 83.3 %, the committed tree counts 83.2 %; the 83.3 only held with another agent's uncommitted emit_core_leaves.nika in the tree) - step bootstrap_compiler, `badges.py --check`
Fixed: none by me: 83b6aac0 (another session) already regenerated the badge to 83.2 %, identical to my fix
Tests: `scripts/badges.py --check` on a `git archive HEAD` export with the fix: `--check` green (rc 0); no Rust changed
Found: none. The agent holding the uncommitted emit_core_leaves.nika change must rerun scripts/badges.py when it commits (the share moves to 83.3 %).
Files: docs/briefs/qa/qa-task.md.report.md
Commit: qa: the self-hosted badge matches the committed tree (83.2 %)
Changelog: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 7b3c9c5ef7501269b149c2f4a581c25ddfb63df3
