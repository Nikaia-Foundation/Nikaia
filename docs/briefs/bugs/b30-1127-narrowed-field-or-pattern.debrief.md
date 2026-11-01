1. **What cost the most:** the probe's variant case. I wrote it with string literals, so it failed for a reason unrelated to #1127. Isolating that took four `nikaia run` probes (p2 to p5) and a re-check with built strings. The fix itself took one build cycle.

2. **What I looked up that the brief could have handed me:** only small things. The facts `set_positions` is keyed by the arm's body, and nothing in Rust builds `CheckFacts`, so a new field touches only `check_facts.nika`. The checker leaves or-pattern names untyped (`pattern_parts`), so only the lowering needs the per-alternative sets. The brief's two suspected locations were both right, which saved most of the search.

3. **Checks:**
   - Useful: the old-binary probe up front, the build cycle and fixpoint, the scratch project with the new test plus the 12 related narrowing tests under both settings, the fails-before run, clippy, `pre-push-check` and `badges.py`.
   - Wasted: one probe whose `.to_upper()` does not exist.
   - Gaps I should flag:
     - I ran clippy with `--profile fast`, not plain `cargo clippy --all-targets -- -D warnings`.
     - I did not run `rustup check`.
     - My grep for related `it` tests was cut at `head -10`, so `it` tests on #886/#902 may exist that I never ran.
     - I took the disk figures after committing, so they are only in my answer, not in the report.

4. **One change that would save the most:** have builder.md tell builders to build test-probe strings from a call (`.to_uppercase()`, `f"{i}"`), not from literals alone. A `String` that is only ever filled from literals can be lowered as `&'static str`, which hides or mimics other defects. The *Traps* view-checker entry makes the same point for views; a general line like that would have saved my four detour probes.
