Result: done
Files: tests/compiler/tests/a_text_from_a_method_call_viewed_and_returned_is_owned.nika tests/compiler/tests/a_text_from_a_method_call_viewed_and_returned_is_owned.stdout crates/nikaia-std/src/tools/views.nika crates/nikaia-std/src/tools/buffers.nika crates/nikaia-std/src/tools/tether.nika crates/nikaia/src/contracts/keep.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/views.rs crates/nikaia-std/src/tools/lowered/buffers.rs crates/nikaia-std/src/tools/lowered/tether.rs
Commit: lowering: text a method hands back as its own is this frame's buffer, and reaches no parameter (#933)
Body: A local bound to a method call whose every method of that name in the package hands back owned text (and `std` has no method by that name) is now a buffer of the frame, as one bound to a free function's call already was (buffers.nika `made_here`, `text_by_every_method`; BufferAsk gains `library_methods`). A view of it that leaves the function lives in the caller's keep (ADR-353 D10), so `tag` and `tag2` lower `-> &'k str` with `__keep`, exactly as `let hex = make(seed); return hex.trim()` does, rather than `-> String`. In views.nika `rsrc_method`, such a method's result reaches no parameter where the receiver's type (a type parameter, a local bound without a type) or its key is unknown (`Unit::owned_results`); before, a type parameter's call reached every argument (`ref(seed, host)`) and a local `Plain {}` receiver gave `ref(seed)`. `fn first(s: String) -> String { return s.trim() }` still lowers `-> &str` over `s`; a method returning a view of `self` is not taken as owned (its ledger result is a view).
Fixes: #933
Tests: new test a_text_from_a_method_call_viewed_and_returned_is_owned failed on the unfixed compiler (rustc: Cannot return value referencing local variable `hex`, both functions), passes now; nikaia test --both-settings --project tests/compiler 252 passed; it views text_tiers lent + guard set 209 passed; clippy clean; pre-push-check ok; lower-std fixpoint (second run unchanged)
Gate: green
Deviation: step 2 - tag and tag2 lower `-> &'k str` with the caller's keep (ADR-353 D10, as for a free function's result), not `-> String`; the output is ab/cd
Found: none
Needs: none
Changelog: A local holding text a method returned as its own, viewed and returned (`return host.hex(seed).trim()` through a local), now lives in the caller's keep instead of lowering as a view of the parameters that rustc refused.
Owner: none
Disk freed: 0 GB
Rs2nika: none
Pushed: 34519e7ccaef92d16df67893feaf480ced7555ce
