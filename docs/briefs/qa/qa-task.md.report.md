Result: done
Cause: the earlier qa-task report (committed in a0d60512..0b0f2f56) names the archived ADR number of the p557g-parser-rules2 debrief fix; step bootstrap_compiler (check-adr-refs.py, 3 citations)
Fixed: the report is rewritten without the archived number (the debrief itself already cites ADR-279); the check is green
Tests: python3 scripts/check-adr-refs.py green; pre-push below
Found: none (a QA report that names an archived ADR in prose trips the check it reports on)
Files: docs/briefs/qa/qa-task.md.report.md
Commit: QA report no longer names the archived ADR it once fixed a citation of
Changelog: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 5786ccd70b669d6302d06ac3858609ce39698fdf
