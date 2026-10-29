Result: done
Pushed:
Tests: mut_text_pushed_then_trimmed (new; failed unfixed, "Cannot borrow *t0 as mutable"), compare_kept_and_owned (new; the #910 probe, failed unfixed per the issue: can't compare &String with String); tests/compiler --both-settings 348 passed; it guard set 23 passed
Deviation: #915 and #910 have one root and one commit
Found: none
Needs: none
Changelog: A text cut by `trim_start_matches` / `trim_end_matches` / `trim_matches` and copied is text of its own, not a view that keeps its source (#915, #910). Decided: every trim method counts as a cut of its receiver in the keep plans, as `trim` did.
Gate: green
Files: crates/nikaia-std/src/tools/buffers.nika crates/nikaia-std/src/tools/lowered/buffers.rs tests/compiler/tests/mut_text_pushed_then_trimmed.nika tests/compiler/tests/mut_text_pushed_then_trimmed.stdout tests/compiler/tests/compare_kept_and_owned.nika tests/compiler/tests/compare_kept_and_owned.stdout assets/badges/self-hosted.svg docs/briefs/lane-e.md.report.md
Commit: A copied trim_*_matches cut is text of its own, not a keep
Fixes: #915, #910
