1. **What cost the most:** waiting. The full `it` binary took about 15 minutes (934 s) and `tests/compiler --both-settings` about 10. My old-vs-new `check` comparison over examples, packages and tests/{build-time,guide,language} hit the 20-minute background limit. Its output went through `grep`, so nothing was saved and the whole run was lost.
2. **What I looked up that the brief could have handed me:**
   - That `nikaia check FILE` inside `tests/compiler` checks the whole project, so a probe file has to be copied out first. One wasted call.
   - That `nikaia test` has no name filter, so "run the 22 assert tests by name" cannot be done literally.
   - That no `tests/compiler` test sets `unproved-claims` through a `nikaia.toml` (a grep).

   The brief's locations and its decision were exact. The fix took one edit and nothing beyond it.
3. **Checks run:**
   - Useful: the probe on the old and new binaries, including the refuted, carried-back and proved variants; the old/new compare of the 22 assert tests (quick and conclusive); `tests/compiler` (484 passed); the filtered `it` run, which found 3 of the 4 failures in 20 s; the full `it` run, which found the fourth.
   - Wasted: the project-wide compare (killed, no output); the first `proj.sh` attempt (`check` needs a FILE argument). clippy 1.97 passed and 1.99.1 was not run; I judged the lowered change too small to need it.
4. **One change that would have saved the most:** Step 2's filter should name `prove_through_loops` and `unsafe_follows_switches`. Better still, the brief could say: "a precondition fix changes what is refused, so `grep -rln 'assert(' crates/nikaia/tests/it/` and run those modules". Then the 4 refusals show up in one minute, not after a 15-minute full run. The brief should also say ahead of time what to do with a true unproved call in a Rust test: edit its program, or stop. That would have let me finish `done` instead of leaving develop red.
