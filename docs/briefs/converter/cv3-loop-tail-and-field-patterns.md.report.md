Result: done
Files: see the three step reports
Commit: rs2nika: a loop tail if-let, a Some field null arm, let mut tuple (3 commits)
Part: #125
Body: let (mut a, mut b) = .. is written let mut (a, b) = ..; a mix of mut and plain names stays as written (NK1307 names it).
Pushed: local fd97d5465f56510be9087cf079255d3fb780b23d
Tests: nikaia test in crates/rs2nika/convert 149 passed; corpus scores unchanged (219 of 928)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: rs2nika writes let (mut a, mut b) as let mut (a, b)
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
