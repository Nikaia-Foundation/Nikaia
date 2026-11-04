1. **Most time.** The slowest part was fixing the test in `sysroot.rs`. I guessed at Nikaia syntax and needed about five edit-and-compile rounds. Each round recompiled `nikaia` and took 40 seconds or more, and the first build took 4m39s. The guesses were:
   - quote escaping in Rust string literals (two sed attempts that did not match);
   - the type is `String`, not `text`;
   - returning a parameter bare gives NK1104, because parameters are `ref` by default;
   - finally, `mut v: Vec[String]` with `v.push(s)`.

   Finding the cause took only a few calls.

2. **Lookups the brief could have handed me.**
   - The brief's two named input files were not where it said. `qa-input.txt` was in `docs/briefs/views/`, not in `qa/`, and I did not read `.claude/agents/qa.md`.
   - `qa-input.txt` held CI logs, so I had to grep for the failing test name instead of reading a list of red steps.
   - Tool-file syntax (`String`, not `text`; how to write an inferred keep) was never stated. A one-line pointer to `docs/builder-facts.md` would have covered it.
   - `nikaia` was not on PATH. The binary is at `target/release/nikaia`.

3. **Checks.** I ran the failing test, clippy on the `nikaia` crate, fmt and `pre-push`. All were useful. Three of the test runs were wasted on my own syntax guesses. `pre-push` was red on the badge, which is not from my change. I did not read the previous commit's report beyond the reason for the red, and that was enough.

4. **One change.** Add a driver-written "failing tests" line to `qa-input.txt`, with the exact `cargo test` command and the path of the binary. It would also help to put a short, working example of an inferred-keep in a tools-file test string into `builder-facts.md`.
