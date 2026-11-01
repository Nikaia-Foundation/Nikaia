Result: done
Files: .claude/agents/architect.md docs/architect.md scripts/drive.nika scripts/cycle.nika scripts/agent.nika tests/compiler/tests/drive_actions.nika tests/compiler/tests/drive_actions.stdout tests/compiler/tests/agent_script.nika tests/compiler/tests/agent_script.stdout tests/compiler/tests/batch_frontmatter.nika tests/compiler/tests/batch_frontmatter.stdout docs/supervisor.md CLAUDE.md .claude/agents/builder.md docs/priorities.md docs/long-jobs.md docs/tokensaverstrategy.md docs/consultant.md docs/builder-facts.md docs/bootstrap-week.md .github/workflows/nikaia.yml docs/briefs/remove-architect.md.report.md
Commit: Remove the architect role: the supervisor writes the briefs, the driver stops for it
Body: drive.nika loses drive_architect; CUT, BATCH DONE and a RED with no builder queued now wait for the batch's suite and stop with `ESCALATE <stop line>: the supervisor writes the next briefs` (drive_stop); a RED with a builder queued goes on with it (the fix-forward brief); the focus rotation skips an empty area instead of asking. cycle.nika prints `-> SUPERVISOR`; agent.nika drops the architect tool set. docs/architect.md moved into docs/supervisor.md (roles, language decisions, filing defects, lanes, briefs and reports, batches, stages, State) and deleted with .claude/agents/architect.md; links repointed; `Needs: architect decision` is now `Needs: supervisor decision`.
Pushed: local 174a6c7a12f891d8449c7d45238a9a32ec2f9998
Tests: drive_actions, agent_script, batch_frontmatter (run standalone from a scratch copy, outputs re-blessed for the removed step; new case `red, nothing queued`) - pass; nikaia check of drive/cycle/agent.nika - pass
Deviation: none
Found: none
Needs: none
Changelog: The architect role is removed: the supervisor writes the briefs and decides obvious language gaps, and the driver stops for it on CUT, RED and BATCH DONE.
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
