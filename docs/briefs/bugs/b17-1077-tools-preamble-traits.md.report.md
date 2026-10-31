Result: done
Files: crates/nikaia/src/sysroot.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/runtime/lowered.rs
Commit: sysroot: the tools preamble brings the prelude's method traits (Fixes #1077)
Body: the generated preamble of the tools and runtime packages now imports DurationExt, DurationIn, InstantSince, LowBits and ListExt as `_` (Part I 1.3), so `d.in_nanos()` and the like compile. The runtime package shares the preamble, so its lowered.rs changed too. Other prelude traits (ByteCopy, ByteReads) were already there.
Fixes: #1077
Tests: probe `d.in_nanos()` on a `time::Duration` in assets.nika: with the fix lower-std + cargo check -p nikaia-std --features compiler clean, probe removed and lowered files re-lowered clean; not run on the unfixed binary (E0599 as in the issue); clippy, fmt, pre-push-check green
Gate: green
Deviation: none
Found: no Nikaia test reaches the tools preamble
Needs: none
Changelog: The tools and runtime preamble brings the prelude's method traits (`in_nanos`, `since`, `low_bits`, list methods).
Owner: none
Disk freed: 0 GB
Pushed: local ced7217b1b0a2c597ef3c5a3e7ef0189d19ee00a
