1. **Most time went to finding the cause.** The brief's *Locations* pointed at `sysroot.rs` and the tools profile, and that was wrong. The real cause was in `string_type` in `text_tiers.nika`. The tools-profile hypothesis cost about 10 turns. Reading the checker's let and return paths cost about 15 more, before a debug `eprintln` in a lowered file showed that the `let` already had `ty = String?`.

2. **What I looked up that the brief could have handed me:**
   - **The failure is not specific to tools files.** A `String?` field, parameter or result anywhere in a file triggers it, and `nikaia run` fails the same way. Two `nikaia check` probes would have shown this.
   - **Which pass writes the `let` type.** `text_tiers.rs::annotate_item` adds the type node that `string_type` picks (ADR-282 D17). I had to find that myself.
   - **`nikaia test` takes no filter.** I only learned this from the error.
   - **A `nikaia check` of a test file with `fn main` fails to parse.** I did not chase it and used `nikaia run` on a scratch copy instead.

3. **Checks, and which were wasted:**
   - **Wasted:** a probe loop over five tools files. It left the tree dirty, and the `interpreter` iteration ran past the 120 s limit and went to the background. The edits to `libraries.nika` and `modules.nika`, and the debug prints in two lowered files, were each restored by name.
   - **Useful:** the probe matrix (`pub`, private, with and without a `String?`). It showed that `String?` plus `pub` was the trigger.
   - **Final run:**
     - `tests/compiler --both-settings`: 248 passed, 1 failed. The failure is `fmt_keeps_a_nested_tuple_closure_whole`, from 7c1643d (it prints `1` without a newline, the `.stdout` has one). I reported it, did not fix it, and did not check it against a clean tree.
     - Clippy and `pre-push-check` were clean.
     - The `it` guard set and `nullable`/`text_tiers` passed.
     - I did not run `cargo test --lib` for `nikaia-std`, because I changed no Rust under `crates/*/src/`.

4. **One change to the brief or builder.md:** add a rule to builder.md: *"For a wrong refusal of a local's type, bisect the program first with `nikaia check` on a few lines, adding one declaration at a time. Then put one `eprintln!` on the binding in a lowered file, and restore it by name."* That is far cheaper than reading the checker. Also say in builder.md that `nikaia test` has no name filter, so a single test program is run with `nikaia run` on a scratch copy.
