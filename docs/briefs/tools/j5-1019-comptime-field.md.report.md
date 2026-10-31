Result: partial (step 1 built; steps 2 and 3 blocked outside the allowed files)
Files: crates/nikaia-std/src/tools/check_core_comptime.nika crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/emit_stmt.nika crates/nikaia-std/src/tools/lowered/check_core_comptime.rs crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/emit_stmt.rs tests/compiler/tests/a_comptime_reads_the_reflected_field.nika tests/compiler/tests/a_comptime_reads_the_reflected_field.stdout
Commit: A comptime in an unrolled turn reads the turn's field name
Body: In a turn the checker writes `f.name` of a meta::Field local as the field's name and works the comptime out once per turn (new CheckFacts.turn_comptime_values, keyed "<start>:<type>:<field>"); the emitter reads it by walk.specialising/at_field. The walk with no field yet neither computes nor refuses such a comptime. No .stderr pinned `String?` as uncrossable, none changed.
Part: #1087
Tests: new a_comptime_reads_the_reflected_field passes (both settings), failed on old-nikaia (NK1127/NK1152, 3 errors); 50 run: it, a_function_takes_a_reflected_field, reflected_field_ty, all comptime_* and grammar_* ok; nikaia-std --lib 239 ok; lower-std fixpoint clean; fmt, clippy, pre-push-check ok
Gate: not run (driver's)
Deviation: steps 2 (`wire(f)`) and 3 (`T?` crossing) not built; the f"..{f.name}.." form still refuses in a turn (its holes are not reached by the rewrite)
Found: (1) sync.nika `violations_in`/`pausing_call`: an explicit generic call `turn[Contact]()` is Seen::Opaque, so a `sync` caller is refused ("something this compiler cannot resolve") and a comptime calling it gets NK1152. Probe: `fn turn[T: Struct]() -> String sync { for f in T::fields { if f.name == "age" { return f.name } } return "" }` `fn v() -> String sync { return turn[Contact]() }`. This blocks step 2's natural route: a sub-program `fn __value[T]()` unrolling `T::fields` to the turn's field, which reuses the emitter's field copies. (2) Step 3 needs a `null` form in the decoder grammar_decode.nika `gr_decode` (not allowed), beside dump.nika/build_values.nika. (3) An older limit, also on old-nikaia: `fn g() -> String { comptime T = shout("x"); return T }` gets NK1127 inside a non-generic fn.
Needs: architect decision: allow sync.nika (fix 1, then step 2 via a generic value fn in comptime_sub_program) and grammar_decode.nika (step 3), or choose a field-specialising rewriter inside comptime_tree.nika
Changelog: A `comptime` in an unrolled turn over `T::fields` can read the turn's field name, `f.name`, directly or through a function (#1087).
Owner: none
Disk freed: 0 (11G free, cache 3.6G, no new entry over 100 MB)
Rs2nika: none
Pushed: local 417abcfce937c5ef3a195d8dc601cd45efba041f
