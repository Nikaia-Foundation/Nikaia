Result: done
Files: crates/nikaia/tests/it/accepted_sweep.rs
Commit: The sweep asks a program's items for main, and the par_fold entry carries its new hash (fix-forward, sweep red since eeaefb1)
Body: Reproduced over a fresh dump of the it binary (1465 programs): two refusals not on KNOWN, neither one of the brief's three suspects. (1) da08992a6f11ec05, tools/comptime_driver.nika, has no main, but the sweep's `rust.contains("\nfn main(")` matched the sub-program text that cr_main writes (a multi-line string literal), so rustc got a crate with no main (E0601). The file is unchanged since ab60347; it now gets through `nikaia check` and the lowering alone, likely through eeaefb1's check/keep changes (not confirmed: that would mean building eeaefb1^). The sweep now asks the parsed items for `fn main`. (2) d4364f4f2ea3f337 is KNOWN's 018870a09f6d362e (fold_lambdas par_fold over a non-frame, ADR-296 D9), the same program and the same refusal after 79d6e81 rewrote its grammar to `->`; the entry takes the new hash. The index.rs Get<&EitherText> impl (suspect 1) produced no refusal.
Fixes: none
Pushed: 9dc79cb (pushed by drive.nika)
Tests: the sweep over the same dump failed with both entries before the change and passes after it (1 passed); guard set 22 passed; no tests/compiler program, since the fix is in the sweep test and no compiler shape changed
Gate: green
Deviation: the KNOWN edit swaps an entry's hash for the same program and reason (79d6e81's rewrite), which the brief's "remove only" rule does not cover; no entry was added
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0 GB (scratchpad dump deleted; cache 3.9G, disk 3.5G free; new cache entries over 100 MB: rlib/af187e66 264 MB, run-bundle/194302eb 285 MB)
Rs2nika: none
