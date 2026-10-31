Result: done
Files: crates/nikaia-std/src/tools/check_grammar.nika crates/nikaia-std/src/tools/check_takes.nika crates/nikaia-std/src/tools/lowered/check_grammar.rs crates/nikaia-std/src/tools/lowered/check_takes.rs tests/compiler/tests/grammar_a_guard_hands_an_optional_label_by_value.nika tests/compiler/tests/grammar_a_guard_hands_an_optional_label_by_value.stderr
Commit: grammar: a guard that hands a label to a parameter that keeps it is NK2105 (#1050)
Body: Reduced to a 30-line probe (the new test). What makes it fail is that the callee keeps the label, not that the label is optional: `fits(k: ref Node, t: Node?)` binds `t` in a `match` arm, so the checker counts `t` as kept. The lowering then passes `t` by value into the guard closure, which the backend needs to be FnMut ("closure only implements FnOnce"). With a `Node` parameter the callee doesn't keep, the lowering lends `&t` and the program runs, which is why the earlier small probe didn't reproduce it. Whether the action reads `t` afterwards makes no difference. `grammar_fixed` (check_grammar) now checks a guard or bound inside a repeat frame, as the checker already does for a lambda or loop body, so moving a label there is NK2105, the code an ordinary call gets when it reuses `t` afterwards (no copy behind the user's back, as that refusal says). check_takes, a file the brief didn't list, gets the guard's reason line, and the help suggests `t?.clone()` for an optional (`t.clone()` is refused on a `T?`). Takes over the stopped builder's diff and test, with the reason and help lines reworded.
Fixes: #1050
Tests: grammar_a_guard_hands_an_optional_label_by_value (on the unfixed binary `check` passes and the run stops in rustc "FnMut ... only implements FnOnce", so the expected .stderr does not match there); probes: the label not kept by the callee still runs, `t?.clone()` runs; packages/yaml 14/14; gate below (tests/compiler at both settings)
Gate: green
Deviation: none
Found: the plain-call NK2105 help (check_takes.nika, `data_handed_over_is_used_again`) also says `t.clone()` for an optional `t: Node?`, which is refused (it wants `t?.clone()`). Probe: pass `t` to `fn f(t: Node?)` that binds it in a `match` arm, then read `t` again
Needs: supervisor push of 44b8f908 (fast-forward rejected)
Changelog: (step 1, #1049) A built-in that takes one pattern (`peek`, `not`, `intern`, `text`, `recover`) takes a sequence or a choice there, as Part II 10's element table says; they passed `nikaia check` and stopped at the run. (#1050) A guard or bound that hands a label to a parameter that keeps it is refused with NK2105, as inside a lambda: the guard only reads its alternative's labels. It used to pass `nikaia check` and stop in rustc. The help suggests `t?.clone()` for an optional.
Owner: none
Rs2nika: none
Disk freed: 0.04 GB (two scratch binaries); target/nikaia-project-tests 6.5G, 3.9G free on /, no new cache entry over 100 MB
Pushed: 44b8f908 (pushed by drive.nika)
