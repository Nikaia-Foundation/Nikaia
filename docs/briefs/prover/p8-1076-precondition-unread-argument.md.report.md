Result: partial
Files: crates/nikaia-std/src/tools/prover_passes.nika crates/nikaia-std/src/tools/lowered/prover_passes.rs tests/compiler/tests/precondition_unread_argument.nika tests/compiler/tests/precondition_unread_argument.stderr
Commit: prover: a call that reaches a precondition's checked entry does not prove it
Body: `count_checked_calls` now counts `CallReach::Through` calls (an argument the prover doesn't read, ADR-269 D20) as well as `Checked` ones. So such a precondition is listed as "one call checks it when it runs" or "refused ... one call does not prove it", not as "proved at every call", and the default build refuses it with NK1300 (ADR-380 D1). The texts in `check_facts.nika` are already right for a `Through` call and are unchanged. The refuted call `at(xs, 7)` (NK1207), the carried-back `pick(xs, 2)` and the proved `at(xs, 1)` read the same on the old and the new binary.
Fixes: #1076
Pushed: local 788981d9bb2110678bca7b38f8c894f577ec6e53
Tests: new test precondition_unread_argument (refused, NK1300): the unfixed binary (nikaia-suite) accepts it, `Checked`, and lists it as proved at every call. tests/compiler --both-settings: 484 passed, 0 failed. All 22 other tests/compiler programs that hold `assert(` give the same `check` and `--asserts` output on the old and the new binary. Whole `it` binary: 2883 passed, 4 failed. The 4 are prove_through_loops::a_break_guard_carries_its_bound_through_the_loop, prove_through_loops::a_break_guard_does_not_hold_past_the_loop, unsafe_follows_switches::with_every_switch_off_the_emitted_code_holds_no_unsafe and unsafe_follows_switches::with_the_switches_on_each_unsafe_is_a_call_into_proven, now refused with NK1300. Each is a true unproved call: `scan(Bytes(v), 10)` and `f(Bytes(v), ...)`, with `v` filled by a `for` loop of `push`, so the prover does not read `buf.len()`/`b.len()` and the call reaches the checked entry. No warn test: no tests/compiler test sets `unproved-claims` through a `nikaia.toml`. The old/new check comparison over examples, packages/json, tests/build-time, tests/guide and tests/language was stopped at its time limit before it printed anything. clippy 1.97 clean; not run with 1.99.1. fmt, pre-push-check and lower-std with its fixpoint are clean.
Gate: red
Deviation: none
Found: none
Needs: architect decision on the 4 newly refused `it` tests (true unproved calls; per the brief I stopped instead of editing them). Either give their programs a length the prover reads (e.g. build `v` without a loop, or add `assert(buf.len() >= 40)` in `main`), or lower them with `unproved-claims = "warn"`. Until then `develop` is red on these 4.
Changelog: A call whose argument the prover cannot read no longer counts as proving the callee's precondition: `--asserts` lists it as not proved, and the default build refuses it with NK1300 (#1076).
Owner: none
Disk freed: 0 GB (cache 5.6G, disk 5.7G free; new cache entry over 100 MB: run-bundle/3f8b93b88f7f213d 480 MB)
Rs2nika: none
