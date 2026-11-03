1. **What cost the most.** The 9-minute `tests/compiler --both-settings` run cost the most time. The most turns went to working out how to run one test program on its own. `nikaia run tests/X.nika` refuses because it is not the entry point, and `nikaia test` has no name filter, so I wrote a scratch-project script. Finding where a type's parts are stored took about 5 turns: `state.structs`, `enum_payloads` and `world.parts_beside`.

2. **Lookups the brief could have handed me:**
   - The text of ADR-388 D2. It is short, so it could have been quoted.
   - Where the catalogue entry goes. The brief says "page", but it is one long table row in `30-nikaia-tooling.md`, inserted before `NK1331`.
   - The type-parts maps above, and `any_takes_space` in `check_decls.nika` as a walk to copy.
   - The `check_refusal` signature, and `Ty::owned()`.
   - That a `.stderr` ends with the `N type errors` line.

3. **Checks I ran:**
   - Needed: the probes on the old binary (they caught the owned-table NK1102 defect and the `Symbol?` defect before I built anything), two build cycles, the `it` tests on `errors::` and `grammar`, clippy, fmt and `pre-push`.
   - Mostly wasted: the full `tests/compiler` run. It was 9 minutes, and the brief asked for it while builder.md says to run only touched tests. The three changed programs had already passed by probe.
   - A real slip: I piped `pre-push` into `tail`, so its exit code did not stop the commit. `dfb6502e` went in while `pre-push` was red on the out-of-date self-hosted badge. That badge is a known trap and yours to fix, but the check was not gating.

4. **One change:** add a documented way to run a single `tests/compiler` program, for example a `nikaia test --only NAME` filter or a `scripts/one_test.nika NAME`. Also settle in the brief whether to run the full `tests/compiler` or only the touched programs, because the brief and builder.md contradict each other.
