Result: done
Files: docs/formal-verification.md
Commit: formal-verification: the proved share measured over the tools package as it lowers (#1053)
Body: §2 and §4 quote `nikaia lower-std --bounds` summed by scripts/proved.nika on de2af5b2: 1,503 of 2,608 indexes proved (57.6 %), 719 of 1,601 arithmetic steps (44.9 %), 246 files, none refused; at prove-through-loops:on 1,535 (58.9 %) and 726 (45.3 %). §5 says how to run the counting tool. Steps 1-2 are de2af5b2, step 3 is 8aed3f6 (message amended locally from Fixes to Part of #1053 before any push, since the brief is step 1 of #1053).
Part: #1053
Pushed: 8aed3f6 (pushed by drive.nika)
Tests: lower-std --bounds on the checkout: 246 files, 0 refused; 2,608 indexes / 1,503 proved, 1,601 operations / 719 proved, which is at least the per-file figure (1,821/965, 1,381/560); logic_search.nika `indexed.atoms[atom]` (line 64 today) is listed; proved+unproved site lines = 2,222+1,987 = every site, reasons add up to 1,987; cargo test --lib sysroot:: bounds_report:: 8 passed; a tests/compiler program is not possible for a sysroot command
Gate: green
Deviation: none
Found: none
Needs: rebase and push of de2af5b2 and 8aed3f6 (both pushes rejected)
Changelog: `nikaia lower-std --bounds` prints the `--bounds` report of each tools file, checked with the package beside it as lower-std checks it (#1053)
Owner: none
Disk freed: 0 GB; target/nikaia-project-tests 529M, / 26G free; new cache entries over 100 MB: two rlib entries of 265 MB
Rs2nika: none
