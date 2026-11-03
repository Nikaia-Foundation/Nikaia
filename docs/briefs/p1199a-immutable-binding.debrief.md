1. **Most turns and time.** The full `it` run took 27 minutes. I also spent two extra turns on wait scripts: the first wait call hit its 10-minute foreground limit, went to the background and was killed there. After that, the work was mostly checking what the earlier, unfinished run had left in the tree, since nothing was committed and the brief didn't say a draft was there. Finally, `nikaia fmt` changed the indentation of 8 test files, so I had to regenerate their `.stderr` files.
2. **What I looked up that the brief could have given:**
   - That an earlier run had left the work uncommitted (I learned it from the debrief and `git status`).
   - That the live grammar is the Rust `grammar!` in `parser/mod.rs`, not `parser_grammar.nika`.
   - That `RESERVED_WORDS` lives in `parse_errors.nika` and a drift test ties it to Part I 2.1.
   - That `tests/specification/EXPECTED.txt` changes whenever a spec example starts to parse.
3. **Checks:**
   - Useful: one `lower-std`/build/`lower-std` cycle plus a fixpoint hash; the 12 tests in a scratch project; `nikaia check` per file from a scratch directory to regenerate `.stderr`; clippy, fmt, `parser::` unit tests, the full `it` run (it found the spec guard) and pre-push.
   - Wasted: the first wait call (it timed out), and the `?? return null` probe could have been run before the build, not separately.
4. **One change.** When a brief re-runs after a broken session, start it with: "a draft is in the tree, these files; check it whole first". Also add to `builder-facts.md`: "a parser or keyword change: regenerate `tests/specification/EXPECTED.txt` with the specification example and commit the diff", plus the command for a `.stderr` (run `nikaia check tests/NAME.nika` from a scratch directory).
