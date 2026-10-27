Result: done
Files: crates/nikaia-std/src/tools/buffers.nika crates/nikaia-std/src/tools/lowered/buffers.rs tests/compiler/tests/string_empty_then_moved.nika tests/compiler/tests/string_empty_then_moved.stdout
Commit: buffers: a clone of a cut of a text copies text, so a String moved into it stays owned (Fixes #881)
Fixes: #881
Pushed: 0fa94bb21fd57b006f3fa3ff1a5717c8d6338b77
Tests: reproduced on develop 3b975cf (Mismatched types at `st = after`); minimised: the trigger is `st = st[f + " for ".len()..].clone()`, not the `""` start nor the push loop (variants without the loop, `after = "x"`, `after.push_str(st.trim())` were owned already); the unfixed binary lowered the minimised String-parameter form into `__keep_frame.put` (the test file itself not rerun on it, the binary was rebuilt); new test prints `<T>` (the issue's `Foo<T>` was a slip: no ` for ` in the input, the name ends at the first space); nikaia test --both-settings tests/compiler 152 passed; it keep view move + guard set 366 passed; clippy clean; pre-push-check ok
Gate: green
Deviation: none
Found: a reassigned String local whose trimmed view is the function's result is refused by rustc: `fn tail(head: String) -> String { let mut st: String = head.clone()  let mut after: String = ""  st = after  let n: String = st.trim()  return n }` lowers `st = __keep.put_viewed(after)` (Mismatched types); present before this change, needs an issue
Needs: none
Changelog: A `String` local moved into a variable that later takes `x[a..].clone()` of itself is lowered as a move; rustc refused it before (#881).
Owner: none
Disk freed: 0 GB (no clone, no scratch target)
