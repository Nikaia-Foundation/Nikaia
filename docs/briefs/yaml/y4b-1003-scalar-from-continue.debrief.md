1. **What cost the most.** Finding out why an unproved `scalar::from` was never refused. Over several probes:
   - My first probes used values the prover pins by running the call at build time (`opaque(xs)`, `r(64, 1)`), so they all came out proved.
   - The real cause was the first run's lookup order: `std`'s contract was cached alongside other packages' contracts, so on the second pass the call no longer counted as `std`'s.
   - After that, deciding how a call site becomes a claim (`std_claims` passed from the walk through `prove.rs` into the checker's claims).
   - Then the json `\u` path: I built step 3 before checking json could be proved, and had to take step 3 back out.
2. **What the brief could have handed me:**
   - NK1300 for a precondition is raised at the callee's `assert` (`check_claims.nika`), counting only `CallReach::Checked` calls. A call whose argument the prover can't read is never refused there.
   - A call is carried back to the caller's entry only at the top of the body, not inside a loop. That is the only quick way to write an unproved probe.
   - `--asserts` lists only the checker's `facts.claims`.
   - `check-adr-refs.py` rejects "§5" (it wants `D5`).
   - Whether `scalar::from(cp)` in json can be proved at all. One probe before the brief was written would have shown that it can't.
3. **Checks run:**
   - Useful: single-file `check` / `--asserts` probes, json and yaml package tests, the guard tests, `tests/compiler` at both settings, clippy, the pre-push check.
   - Wasted: the first `tests/compiler` run, killed once I took step 3 out. My `pkill` also killed my own shell. A converter build that cannot run without rust-analyzer, and it dirtied four tracked lock and contract files I then had to restore. A `grep -r` over `target/` that hit the timeout.
   - Process note: I unstaged my own `git rm` with a path-limited `git reset -- <paths>`, and pushed step 3 to a wip branch through plumbing (`commit-tree`). Both touched only my own files, but neither is in builder.md.
4. **The one change:** require a feasibility probe of every call site the brief names (here json's `\u` line, run under `--asserts`) before writing a step that depends on it. Add one line to builder.md: "To probe an unproved claim, put the call in a loop body; a value from a pure call is run at build time and proved."
