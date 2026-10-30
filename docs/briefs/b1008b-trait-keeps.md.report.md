Result: done
Files: crates/nikaia/src/contracts/keeps.rs tests/compiler/tests/a_trait_method_callback_is_handed_on.nika tests/compiler/tests/a_trait_method_callback_is_handed_on.stdout
Commit: contracts: an impl's method keeps what its trait method keeps (#1008)
Body: keeps::infer first reads every unit's trait methods for the positions they keep (a by-value, non-mut parameter, the Item::Trait arm's rule), then unions those positions into the uses of each method of an `impl Trait for T` before the lends fixpoint; the emitter and the checker's kept_args read the column unchanged. Traits from the library ledger are not looked up (no tools file needed it; lower-std fixpoint changed no lowered file).
Fixes: #1008
Pushed: d1ff2ec3 (supervisor rebased 0f5c5a52 and pushed)
Tests: new a_trait_method_callback_is_handed_on (6, 3, 9) failed on the unfixed compiler (E0049 type-parameter mismatch, Mismatched types) and passes now; tests/compiler --both-settings 363 passed; it guard set + keeps 155 passed; nikaia --lib contracts:: 36 passed; clippy clean; lower-std fixpoint clean
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: Programs that hand a callback through a trait method now run: an impl's method keeps the fn parameters its trait method keeps (#1008).
Owner: none
Disk freed: 0 (project-tests cache 4.0G, 3.3G free on /; new entry run-bundle/b2db8b4e959f1cfc 347 MB)
Rs2nika: none
