---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Remove the architect role (owner, 2026-11-01)

The owner removed the architect: the main session (supervisor) writes the
briefs, keeps the lanes and the issues. A gap of kind 2 (CLAUDE.md) the
supervisor decides when it is an obvious language change by the language
principles; anything else goes to the owner as a `decision` issue.

Remove the role and keep what is still a rule:
- .claude/agents/architect.md: delete.
- scripts/drive.nika (drive_architect, the `architect` action and its
  callers), scripts/cycle.nika, scripts/agent.nika: take out the architect
  step; the driver goes on with briefs the supervisor writes. Keep their
  tests passing (drive_actions, agent_script, batch_frontmatter), changing
  them only for the removed step.
- docs/architect.md: move what still holds (lanes, language decisions,
  briefs and reports) into docs/supervisor.md, then delete the file;
  repoint every link to it.
- CLAUDE.md, docs/supervisor.md, docs/priorities.md,
  .claude/agents/builder.md, the workflow yml comment: replace "the
  architect" with the supervisor or the owner as above.
Historical briefs, reports, ADRs and the CHANGELOG keep their mentions.
No version raise (docs and tooling). Two builders fix compiler defects in
parallel in the same checkout.
