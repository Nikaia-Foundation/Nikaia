1. **Most time:** waiting on long checks, not working out what to build. The whole `tests/compiler --both-settings` run took about 10+ minutes and the `it` filters 278 s. A `cargo check` of nikaia-std ran past the 120 s default, went to the background and cost an extra round-trip. The brief's line numbers were exact, so the code itself took about 5 turns.
2. **Looked up:**
   - How `time::Duration` is written in `std.contracts`, and whether it has `compares` (for `<=` in the test). I grepped for it.
   - Whether any contract already returns a tuple option (`(..)?`). It does: `split_once`.
   - How ADRs write an amendment. No ADR has an "Amendment" heading, so I made up `## N. Amendment (#922, date)`. The brief could name the form.
   - That ADR-368 section 4 rejects the wall clock outright. The amendment has to narrow that rejection; the brief didn't say so.
3. **Checks:**
   - Ran: unfixed-binary probe, build, `lower-std`, `nikaia run` probe, full `tests/compiler` both settings, nikaia-std `--lib` tests, `it` filters, clippy on 1.97 and 1.99.1 (each `-p nikaia-std`), `pre-push-check`.
   - Wasted: the separate `cargo check` before the build, since the build checks the same code. Also running `pre-push-check` twice: its silent output didn't show the exit code.
   - The `it` filters `fs metadata time` were probably overkill for a change that only adds things.
4. **One change:** builder.md should say to run a pre-build `cargo check` with `timeout: 600000` or skip it when only `std` Rust changed. It should also say that `pre-push-check.sh` prints nothing on success, so callers should append `; echo rc $?`. For a brief like this one, it could also list the one ADR line its amendment overrides (ADR-368 §4 "A wall clock").
