Result: done
Cause: 788981d9 (p8, #1076) - test red: prove_through_loops::a_break_guard_carries_its_bound_through_the_loop, a_break_guard_does_not_hold_past_the_loop, unsafe_follows_switches::with_every_switch_off_the_emitted_code_holds_no_unsafe, with_the_switches_on_each_unsafe_is_a_call_into_proven (NK1300: `Bytes(v)` has no length the prover reads, so the call is now rightly refused)
Fixed: those two test files lower at `--unproved-claims warn`; they test bounds and emitted `unsafe`, not whether the call is proved. p10 (#1084) still builds the length rule.
Tests: cargo test --profile fast -p nikaia --test it -- prove_through_loops unsafe_follows_switches: 11 passed, 0 failed (was 4 failed). clippy, pre-push-check clean.
Found: none
Files: crates/nikaia/tests/it/prove_through_loops.rs crates/nikaia/tests/it/unsafe_follows_switches.rs
Commit: tests: the loop-bound and unsafe-switch programs lower at unproved-claims warn (p8 refuses their unread Bytes length)
Pushed: local 2a764f6551dba5640e82435dbea016ebe74a98a4
