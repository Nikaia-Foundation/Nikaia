1. **What cost the most:** checking the supervisor's six defect reports one at a time, which took about 15 turns. Several first probes were wrong: `Duration::from_millis` and `Duration::new` don't exist, and the right form is `time::Duration(30, 0)`. Two of the reports didn't reproduce. Repo-wide GitHub search is blocked, so I had to page through the open issues to check for duplicates.
2. **Looked up that the brief could have handed me:**
   - Where ADR-361 lives (`docs/specification/adr/`). I tried `docs/adr` first.
   - The headings of D5, D10, D11 and D12.
   - How `nikaia test` is tested: the temp-project pattern in `a_library_trait_with_no_impl_runs.nika`.
   - That `nikaia.toml` has no optimization key.
   - Which file each line of the `lower-std --bounds` report belongs to: file headers start in column 0, site lines are indented.
   - The probe paths from the old scratchpad, and that `async e` is #984's planned build, not a defect.
3. **Checks:**
   - Useful: one `lower-std --bounds` run at `on` (about 41 s), the counter probe at both settings, the NK2305 probe (a `for` cannot change what it walks), the Part I hole rule, and P5c's `Cut:` range against `project.rs`.
   - Wasted: the first Duration probes, the async probe that ran `.await` before the plain one, the search API calls, and greps for an optimization key in the toml.
   - Not run: the count at the `off` setting. I took those figures from p4's report.
4. **The change that would have saved most:** a supervisor note handing over a defect should already hold the probe text and a "checked on <sha>" line, and say whether it is a planned build (an issue number). The architect then files it without re-probing. I would add this rule to `docs/supervisor.md` and to the task template, and also put an *ADR paths and report format* line into the lane facts in `state.md`.
