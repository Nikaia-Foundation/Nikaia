1. **Time and turns.** The run was short, about 12 tool calls. The most expensive step was the first full read of `describe.nika` and `project_members.nika`. One `Read` returned a 53 KB dump I had to re-read in pieces. The build then failed on #1144, which the brief said b36 would already have fixed.

2. **What I looked up that the brief could have given me.**
   - The type of `ledger.sources` and `Sources.files`. They turned out to be plain `BTreeMap<String, String>`, but I only checked this late.
   - Where the probe files were. They sit in another session's scratchpad (`b7e2d91b…`), and I found them with `find /`.
   - Whether the b36 fix was in my binary. The brief said it would be, but nothing told me how to check, and I only learned it wasn't from the failed build.
   - Two forms the brief didn't cover: comparing `words.next()` against a `String` (NK1102), and assigning an owned string to a `let mut` that started as a literal (NK1105). Each cost one `lower-std` cycle.

3. **Checks.**
   - `lower-std` with the old binary: it caught the NK1102, NK1105 and NK2702 refusals in seconds. That was useful.
   - `cargo build`: it found the #1144 failure. Useful, but it came after I had written the Rust side too.
   - I never reached the tests or clippy.
   - Wasted: the Rust forwards and the `self_hosting.py` row, which I wrote before knowing the build would fail. I reverted them.
   - Wasted: the large `describe.nika` output.

4. **One change.** Give a brief that depends on an unlanded fix a pre-flight line, like the "match by subject" rule: `git log --grep=1144` plus a 3-line probe against the current `target/fast/nikaia`. If the probe fails, the brief should say stop before any edits. That would have shown the block in 2 turns instead of 12, and left no revert.
