1. **Time cost.** The two `cargo test -p nikaia --test it` runs cost the most. The first one included a 4-minute build, and the later runs each took about 15 s of test time. A broad `grep -rnw immutable` over `crates tests guide`, which pulled in `target/`, hit the 120 s timeout and went to the background. It cost one wasted call and left a background task running.

2. **Lookups the brief could have handed me.**
   - The brief points to `qa-input.txt`, but its first lines already name only the CI job. It does not say that `suite.log` holds the failing test names near the end, so I grepped for them.
   - I had to find that the unpushed commits were spec-only (9ebcd115 and 1755150b).
   - I had to find where `RESERVED_WORDS` lives and that the real reservation is a grammar rule, not just the list.
   - I had to find that `nikaia` is not on PATH and `target/debug/nikaia` has to be used.
   - I had to find the report fields and the commit script's `Changelog:` and `Attribution:` format.

3. **Checks.**
   - Useful: the targeted `specification::` tests, the numbers-stripped diff of the two baselines, and `pre-push.nika`.
   - Wasted: the repo-wide grep above.
   - Not run: `from_is_a_name`, since it cannot pass without the parser change.
   - I did not run clippy because I changed no Rust.

4. **One change.** Add a short "first five minutes" note to `qa.md`, covering three things:
   - Run `git log origin/develop..HEAD --stat`, because spec-only commits that turn tests red usually mean a stale baseline or an unbuilt feature.
   - Say that `target/debug/nikaia` is the binary.
   - Say that a page list of reserved words ahead of the parser is a supervisor decision (blocked), so QA doesn't spend turns on it.
