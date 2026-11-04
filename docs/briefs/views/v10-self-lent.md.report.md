Result: partial
Files: crates/nikaia-std/src/tools/check_core_args.nika crates/nikaia-std/src/tools/check_core_leaves.nika crates/nikaia-std/src/tools/lowered/check_core_args.rs crates/nikaia-std/src/tools/lowered/check_core_leaves.rs crates/nikaia/tests/it/borrowed_subject.rs crates/nikaia/tests/it/generics.rs tests/compiler/tests/views_a_field_of_self_lent_to_a_plain_parameter.nika tests/compiler/tests/views_a_field_of_self_lent_to_a_plain_parameter.stdout tests/compiler/tests/views_a_field_of_mut_self_given_to_a_keep_parameter_is_refused.nika tests/compiler/tests/views_a_field_of_mut_self_given_to_a_keep_parameter_is_refused.stderr
Commit: check: a field of `self` handed to a lent parameter stays lent (ADR-views-3 D2)
Body: Step 2 of v10. `check_core_args_other` no longer raises NK1131 when the callee lends the parameter: the new `check_core_args_lent` (Nikaia) asks `lends_in`, the same answer the emitter writes the `&` from, for a free function. A method's plain parameter is not inferred lent yet, so it is still refused. NK1131's note and help no longer name `ref self`: the note says `self` and `mut self` borrow, the help offers `keep self`. Changed Nikaia: check_core_args.nika (`check_core_args_other`, new `check_core_args_lent`), check_core_leaves.nika (`check_core_a_field_of_a_borrowed_subject_given`). Rust: no function added. Help-text asserts updated in it/borrowed_subject.rs and it/generics.rs. Step 1 (a bare `self` is lent) is not built, see Found.
Part: #1213
Tests: both new tests/compiler tests pass --both-settings and fail on the unfixed binary; the 8 NK1131/NK2106 tests/compiler programs pass; it borrowed_subject:: generics:: nullable:: defects_found_running:: borrowed_part:: text_tiers:: traits:: keeps:: 242 passed; lower-std at its fixpoint; fmt and clippy clean
Gate: green
Deviation: step 1 not built (see Found)
Found: step 1 is blocked. Reading a bare `self` as `ref self` in the parser (`receiver` rule) works for probe 1, but the copyable `EmitFlow`'s `self with {..}` methods (emit_kinds.nika 308-337) then hit NK1173 (ADR-118: no `with` over a view). Their way out, `keep self`, opts emit_kinds.nika into the NK2302 keep check, which then fires wrongly on `emit_pointer_for(ty: ref Type)`: keep inference reads a field of a struct declared in another file as keeping the parameter. Probe: a 2-file package, src/types.nika `pub struct Type { pub name: String, pub is_slice: bool, pub is_mut: bool }`; src/main.nika `pub enum EP { Run { mutable: bool } }  pub fn pf(ty: ref Type) -> EP? { if ty.is_slice { return EP::Run { mutable: ty.is_mut } } return null }  pub fn k(keep x: String) -> String { return x }`; `nikaia check src/main.nika` gives NK2302 "`pf` keeps `ty`" (with Type in the same file it passes). Opting in also asks for `keep named: bool` / `keep statement: i64` on EmitFlow::handling/at.
Needs: supervisor decision: either fix the cross-file keep inference (keeps.nika) and then build step 1 with `keep self` on EmitFlow's builders, or decide that a bare `self` of a type that derives Copy lowers by value (ADR-views-3 does not say)
Changelog: A field of `self` or `mut self` can be passed to a plain (lent) parameter of a function; NK1131 now names `keep self` as the way out instead of `ref self`.
Owner: none
Disk freed: none
Rs2nika: none
Pushed: local f05974a6c1ccccdaebd2a55b555dd222c59e4978

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
