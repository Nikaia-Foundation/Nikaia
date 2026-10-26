Result: done
Files: crates/nikaia-std/src/tools/check_arguments.nika crates/nikaia-std/src/tools/check_core_call_on.nika crates/nikaia-std/src/tools/check_overflow.nika crates/nikaia-std/src/tools/check_texts.nika crates/nikaia-std/src/tools/emit_core_places.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/check_arguments.rs crates/nikaia-std/src/tools/lowered/check_core_call_on.rs crates/nikaia-std/src/tools/lowered/check_overflow.rs crates/nikaia-std/src/tools/lowered/check_texts.rs crates/nikaia-std/src/tools/lowered/emit_core_places.rs tests/compiler/tests/a_view_of_an_element_as_a_tuple_part.nika tests/compiler/tests/a_view_of_an_element_as_a_tuple_part.stdout tests/compiler/tests/a_view_of_an_element_pushed_into_a_vec_of_views.nika tests/compiler/tests/a_view_of_an_element_pushed_into_a_vec_of_views.stdout
Commit: A view of an element as a Vec element or a tuple part is lent
Part: #655
Pushed: 6c24a3058644c6096231cd754d5f2a7a4a2b3b71
Tests: probes (a)(b)(c) failed on the unfixed binary (rustc E0308 / NK1103) and print 1 now; (d) unchanged; (e) returned Vec[ref E?] from a one-ref-parameter fn lowers right, prints 1; two new program tests; tests/compiler both settings 133 ok; it guard set + element_views views_kept stored_views tethers 85 ok; clippy clean
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A view of an element pushed into a `Vec[ref E?]` or written as a part of a `(i64, ref E?)` tuple is lent and goes inside its option (`Some(&..)`) instead of failing in rustc or NK1103 (#655)
Owner: none
Disk freed: 0
