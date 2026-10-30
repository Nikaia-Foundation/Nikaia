Result: partial
Files: see the step reports (.step1/.step2.report.md)
Commit: 7f90737b scalar::from(n) with a precondition std publishes, held as a claim of the program at each call; 00dc564a json, grammar_arrow: a character written as one, not a literal cast as scalar
Body: 1a and 1b in one commit: std's `requires` reach the prover; the precondition at a std call is a claim like an `assert` (proved, carried back to the caller's entry and then proved at its callers, NK1207, or NK1300 / a warning under "warn"), listed by --asserts. Step 3 (NK1241 refusal, ToScalar removed, Part I/III text, test) is on wip/y4b-1003-as-scalar-refused (dfe7bac6): not on develop, because json's `scalar::from(cp)` is refused NK1300.
Fixes: none
Pushed: rejected (7f90737b, 00dc564a local on develop; origin moved, supervisor rebases)
Tests: tests/compiler --both-settings 360 passed (3 new tests; they cannot pass on the unfixed compiler, which has no scalar::from); it range_spelling shipped_ledger contracts dependency_ledger fmt::every_nika_file unproved_claims prove assert green; lib prove/check, nikaia-std num:: green; packages/json 7/7, packages/yaml 9/9; clippy, pre-push-check clean; warn: `lower --unproved-claims warn` gives warning[NK1300]
Gate: green
Deviation: step 2's json \u line (196) and rs2nika tr_expr.nika not changed; step 3 not on develop (wip branch). rs2nika: Rust `as char` takes only a u8, so `as scalar` stays right; no rust-analyzer here to test a from_u32 rule
Found: (1) json's \u path cannot prove scalar::from(cp): `assert(n >= 0 && n <= 0xFFFF)` before hex4's return is NK1300 (a range accumulated across a 4-turn loop is not proved), and a `let mut cp` reassigned inside an `if` is "not a whole number the prover reads" afterwards; probe: scratchpad y4b/hx2.nika (hex4 loop over [48,48,52,49]; `let mut cp = first; if first == 0x41 { cp = first + 1 }; scalar::from(cp)` -> NK1300). (2) Lowering defect like #943: `let mut why = "a"; if c { why = "b" }; held.push(E::V(why))` where `E::V(String)` gets an owned String elsewhere reaches rustc (E0308, &str vs String); probe y4b/why3.nika; my code pushes each literal directly instead
Needs: owner decision: json's \u decoding needs the prover to know hex4's range (loop invariant) and to read a branch-reassigned `let mut`, or json restructured; then rebase wip/y4b-1003-as-scalar-refused, run lower-std, commit with Fixes #1003
Changelog: `scalar::from(n)` turns an integer into a `scalar` under a precondition the compiler proves at each call or refuses (`NK1300`); `std`'s preconditions are claims of the program like an `assert`, listed by `--asserts`.
Owner: should json's hex4 be rewritten so the prover sees its range, or should the prover learn the loop range first (step 3 waits on it)?
Rs2nika: none
Disk freed: 0.02 GB (scratch binary); target/nikaia-project-tests 5.5G, / 12G free; no new cache entry over 100 MB
