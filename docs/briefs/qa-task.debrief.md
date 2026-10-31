1. **Time and turns:** The first `cargo test` ran in the dev profile. It sat for the 120 s timeout, moved to the background, and then failed with a recursion-limit error. That cost about three turns and several minutes. Before that, I lost a turn looking for `qa-input.txt` in the wrong directory, because the brief names two possible locations.

2. **Lookups the brief could have handed me:**
   - Which lane directory holds `qa-input.txt` (here `docs/briefs/bugs/`).
   - That the test profile is `--profile fast`. It's in `Cargo.toml` comments, and the dev profile failed to compile.
   - That `qa-input.txt` can list no unpushed commits and truncate the failing test names with `...`.
   - That another agent had already moved `HEAD`. By the time I looked, `f2292c44` had fixed the test.

3. **Checks:**
   - **Wasted:** the dev-profile `cargo test` run, which only produced the recursion-limit error.
   - **Useful:** the fast-profile rerun of the two named failures, then the whole `grammar_lowering::` and `examples::` modules. These showed the red was already fixed.
   - **Not run:** fmt, clippy and `pre-push-check.sh`, since I changed no code.

4. **One change:** the driver should write the full failing test names, and the sha it tested, into `qa-input.txt`. It should also tell the QA agent to first check whether `origin/develop` or `HEAD` has moved past the tested sha and whether the failing tests now pass. A line in `qa.md` saying "use `cargo test --profile fast`" would also have saved the dev-profile detour.
