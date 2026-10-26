Result: scripts/tokenspeed.nika replaces tokenspeed.py (same two modes, output byte for byte on 199 transcripts); agent.nika appends the run's Runs row once per agent id; 73 rows backfilled; tokenspeed.py deleted
Files: scripts/tokenspeed.nika scripts/tokenspeed.py scripts/agent.nika docs/tokenspeed.md tests/compiler/tests/tokenspeed_row_formatting.nika tests/compiler/tests/tokenspeed_row_formatting.stdout
Commit: tokenspeed.nika replaces tokenspeed.py; agent.nika appends each run's row to docs/tokenspeed.md (#858)
Fixes: #858
Tests: tests/compiler --both-settings (98 ok, incl. tokenspeed_row_formatting, agent_script); guard set 22 ok; pre-push-check ok; diff vs Python on 199 transcripts equal except 3 live agents that grew
Gate: green
Deviation: (1) the test holds a COPY of ts_div/ts_commas/ts_tenths/ts_per/ts_row (a program cannot use a script). (2) agent.nika no longer passes --no-session-persistence: with it a headless run leaves no transcript, so no row can be made. (3) agent_runs_insert is not `throws`. (4) --short also takes one transcript FILE (a headless run's <session>.jsonl). (5) JSON is read by string search (no std JSON); a line with two objects glued is skipped like Python skips bad JSON. (6) CLAUDE.md, docs/speed.md, docs/tokensaverstrategy.md still name tokenspeed.py (not in my files).
Found: with `throws` on a helper that cannot throw and main calling it, `process::exit(2)` in agent_escalate ends with code 1 and "Error: process_Exit" instead of 2 (agent.nika variant before the fix; agent_script test showed it). Not filed, no short probe made. Also: module detection reads the first 500 chars, a headless/brief-driven agent reads "Read your brief at ..." so almost all builders are "plan" and 0 moved (as in the Python); backfilled rows inherit that.
Needs: supervisor commit takes docs/tokenspeed.md after each agent.nika run; update the three docs that name tokenspeed.py; rows of the 4 unfinished agents (a53e7b, afda42, a7addd, a3666e = this run) not backfilled; module detection from the brief's text would give real moved lines.
Changelog: none (tooling)
rows backfilled: 73 (finished builders of 2026-10-25/26)
GB freed: 0 (cache dir not removed; disk not under 3 GB)
Owner: none
Pushed: rejected
