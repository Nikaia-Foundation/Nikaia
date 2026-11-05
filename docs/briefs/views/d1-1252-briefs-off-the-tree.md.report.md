Result: done
Files: scripts/drive.nika tests/compiler/tests/drive_actions.nika tests/compiler/tests/drive_actions.stdout tests/compiler/tests/drive_issue.nika tests/compiler/tests/drive_issue.stdout tests/compiler/tests/drive_post_body.nika tests/compiler/tests/drive_post_body.stdout
Commit: drive: commit no brief, report or run file; post each brief on its issue once
Body: drive_commit and drive_paths go: after a builder or QA run the driver only rewrites the report's Pushed: line (drive_report_pushed) and commits nothing, so briefs, reports, run.json, debriefs, tokens.txt, cost.txt and docs/tokenspeed.md stay untracked working files (tokenspeed.md is outside docs/briefs/ but was only taken by drive_commit; it now waits for a supervisor commit, as agent.nika's comment already says). The block end writes focus.txt and appends focus-log.md without a commit. QA input already found the batch's briefs and reports from DIR/unpushed.txt (a working file), its headings say so. Before a builder runs, drive_post_brief posts the brief on its issue (drive_issue: the first dash-separated part of exactly four digits after the prefix) through gh api .../issues/N/comments -F body=@FILE, body from drive_post_body (Brief queued: <name>, blank line, brief, blank line, ---, the Generated-by line), and writes the marker <brief>.posted; no issue logs `no issue for <name>`, a failed post `post failed for <name>`, neither stops the batch. Test flags --issue NAME... and --post-body BRIEF. agent.nika and batch.nika commit nothing and stay unchanged.
Part: #1252
Pushed: local 29ea668514d8c545202804704c9526a77c0f6e90
Tests: tests/compiler drive_issue, drive_post_body (new), drive_actions (re-blessed: the dry run shows no git add/commit, the no-issue line and the Pushed: rewrite), drive_docs, drive_rejected_push, drive_changed, focus_rotation: 7 passed; the new tests test the script, not the compiler: the old script has no --issue/--post-body; the real gh post was not run
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: driver: the driver commits no brief, report, run file or focus.txt any more, and posts each brief on its issue once before it runs (#1252 steps 5 and 6)
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
