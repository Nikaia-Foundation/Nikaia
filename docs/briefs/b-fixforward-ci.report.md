Result: green for the five tests
Tests: it: language, sequences, specification, nikaia_test, tuple_let, guard set; tests/compiler both settings 100 ok
Deviation: no new test program: the failing case is tests/language/src/task_scope.nika itself (a panicking scope prints to stderr, which a tests/compiler program cannot hold)
Found: 0c17885c (#853): tuple_let.rs used .to_string() on a literal (NK1301); EXPECTED.txt #86/#87 gain NK1301; a catch whose guarded task::scope body ends in a literal and whose handler is an f-string failed in rustc (&str vs String), fixed in the checker/emitter (owned_catches fact); 7e657b37/85bb29b9 (#857): the Comptime arm of emit_stmt_a_stmt left 2 unanswered calls (pair.0.clone()), now destructured
Gate: green
Files: crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/emit_core_flow.nika crates/nikaia-std/src/tools/emit_stmt.nika crates/nikaia-std/src/tools/lowered/check_core_flow.rs crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/emit_core_flow.rs crates/nikaia/tests/it/tuple_let.rs tests/specification/EXPECTED.txt
Commit: Fix forward: a catch beside a text literal owns its value, tuple_let and EXPECTED.txt follow NK1301, emit_stmt_a_stmt answers its calls
Part: #853
Pushed: 8e86495d14648748504bda91fbff09add342cfe2
