1. **Time and turns.**
   - The biggest cost was waiting on K4's uncommitted `keeps.nika`. It made `lower-std` and `cargo check` fail for the whole shared tree, so I lowered my file in a scratch `NIKAIA_SYSROOT` copy. Then I polled with a background `cargo check` loop until the tree compiled.
   - Smaller losses: a wait loop I wrote badly on the first `lower-std` output, and about six check cycles on small syntax refusals.

2. **What the brief could have handed me.**
   - `use std::i64` is refused (NK1156), and `i64::from` needs a `use` line (NK1117). Widening in `tools/*.nika` is `x as i64`. `builder-facts` says the opposite.
   - `i64::min(a, b)` takes one argument, so it is `a.min(b)`.
   - `grammar` is a reserved word as a variable name.
   - A `comptime` cannot hold a `Vec` of tuples or of structs (NK1167). It can hold `ref Array[Struct]` with `ref String` and `ref Array[ref String]` fields.
   - `for x in v` over an owned `Vec` lowers to a borrow, so it needs `.drain()`.
   - A function that calls itself recursively lowers as `async` unless declared `sync`.
   - `text.bytes.len()` trips `clippy::needless_as_bytes`, which needed an allow-list entry.
   - `nikaia check` on a lone tools file reports false "no type" errors, because it needs the package.

3. **Checks.**
   - Useful: the scratch-sysroot `lower-std`, the `cargo check` and clippy runs, the tests the brief named (8 and 36 passed), `pre-push` (it caught the stale `self-hosted.svg`), and the `lowered.rs` diff.
   - Wasted: `nikaia check` on the standalone file, and the first `lower-std` that failed on K4's file.
   - I did not run the new-binary fails-before test, since the brief ordered no new test. I could not compare the assembled package byte for byte against the old output. Equality rests on `lower-std` leaving every `lowered/*.rs` unchanged.

4. **One change.** The brief should list the lane's dirty files. The builder should be told that a shared-tree refusal in another agent's `.nika` means running `lower-std` with `NIKAIA_SYSROOT` on a scratch copy at once. A pre-flight "tree compiles?" check would also have saved the polling.
