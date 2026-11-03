Result: done
Files: crates/nikaia-std/src/tools/check_core_args.nika crates/nikaia-std/src/tools/lowered/check_core_args.rs tests/compiler/tests/flat_map_seq_result.nika tests/compiler/tests/flat_map_seq_result.stdout docs/briefs/bugs/b21-1097-flat-map-seq-result.md.report.md
Commit: check: flat_map binds its element from a Seq, Vec or Array result (Fixes #1097)
Body: The rule sits in check_core_args.nika `check_core_args_lambda`, beside #1078's `??` rule: where the written result is a `Vec[U]` the receiver left unbound (`Vec[?]`) and the body comes to a `Seq[X]`, `Vec[X]` or `Array[X]` with `X` known, the lambda reports `Vec[X]` (`check_core_args_walked`), so the plain `bind` gives `U = X` and the argument still fits `Vec[$U]`. A first try in ty.nika `bind` (a `Vec[U]` result pattern binding the element of a `Seq`) could not stand: reporting the body's `Seq` makes the argument check (check_arguments.nika, outside the brief's files) refuse it with NK1102, and making a `Seq` fit a `Vec` in `Ty::fits` would be the subtyping ADR-013 D7 refuses. So nothing is accepted that the old checker refused, only `U` is filled in. Other walkables are left as before: the `BTreeSet` lambda in ledger_items.nika:324 still reports `Vec[?]`. `flat_map` needed no `mapped` rule in text_tiers.nika: the `trim()` row lowers `.map(|p| { p.trim() })` with the parts kept as views. The test's named-function row runs over `split`, because a named function over `iter()` fails in rustc (`fn(&&_)`) on the unfixed binary too, with `map` as well (#1098's family).
Fixes: #1097
Tests: new flat_map_seq_result: unfixed binary FAILED (NK1245 twice, the Seq rows), fixed passes with --both-settings; probes on the unfixed binary: Seq row NK1245, Vec row printed 4, named row over iter() a rustc fn(&&_) mismatch (same with map, #1098), over split printed 4; filter_map_split_parts passes; it nullable seq_reductions views_kept map_trust templates 126 passed; grammar_bytes result_source modules package_graph build_time_dependencies 119 passed; lower-std fixpoint clean (the second lower-std changed no lowered file); cargo fmt, clippy clean
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: The function `flat_map` takes may hand back a `Seq` as well as a `Vec` or an `Array`, and the next lambda gets its element's type (for example the parts of a `split`, still as views) (#1097).
Owner: none
Disk freed: 0.05 GB (scratch binaries); project-tests cache 2.6G, disk 3.5G free of 252G; new cache entries over 100 MB from the it runs: build-time-parsers 1025 MB, build-time-bundle 546 MB, run-bundle 288 MB, two rlib 272 and 267 MB
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
