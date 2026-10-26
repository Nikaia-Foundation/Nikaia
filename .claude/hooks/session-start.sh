#!/bin/sh
# Cloud sessions start without a nikaia binary; the main session's scripts
# (scripts/cycle.nika, agent.nika, drive.nika) need target/fast/nikaia.
# Build it in the background so the session starts at once (first build about
# 3 minutes). Log: target/session-start-build.log
[ "$CLAUDE_CODE_REMOTE" = "true" ] || exit 0
cd "$CLAUDE_PROJECT_DIR" || exit 0
[ -x target/fast/nikaia ] && exit 0
mkdir -p target
nohup cargo build --profile fast -p nikaia >target/session-start-build.log 2>&1 &
echo "nikaia: building target/fast/nikaia in the background (log target/session-start-build.log)"
