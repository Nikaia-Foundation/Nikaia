Result: done
Pushed:
Tests: bytes_option_param (new, the #913 probe; failed unfixed per the issue in rustc); tests/compiler --both-settings 348 passed; it guard set 23 passed
Deviation: none
Found: none
Needs: none
Changelog: `want?.clone()` of a `ref Bytes?` is a `Bytes` of its own, and a `ref Bytes?` argument is lent as `Option<&[u8]>` like `ref String?` (#913). Decided: the checker's byte_copies fact holds through `?.` (keyed on the bound receiver), and `Bytes` joins `String` in the `.as_deref()` opening.
Gate: green
Files: crates/nikaia-std/src/tools/emit_core_args.nika crates/nikaia-std/src/tools/lowered/emit_core_args.rs crates/nikaia-std/src/tools/emit_core_calls.nika crates/nikaia-std/src/tools/lowered/emit_core_calls.rs tests/compiler/tests/bytes_option_param.nika tests/compiler/tests/bytes_option_param.stdout docs/briefs/lane-e.md.report.md
Commit: Lower a ?.clone() of ref Bytes? to owned bytes and lend ref Bytes? as a slice
Fixes: #913
