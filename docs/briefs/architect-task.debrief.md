1. **What cost the most turns:** rebuilding the defect probe. BT's report pointed to `bt/probe.nika` in its own scratchpad, which no longer exists. I rewrote the probe from memory and failed four times on Nikaia syntax: how to build a `BTreeMap`, `usize` instead of `int`, a redundant `ref String`, and `.to_string()` on a text literal. Getting a correct, minimal probe took about 6 turns.

2. **What the brief could have handed me:**
   - The probe text itself, not a path to it.
   - Where `known_default` lives: I first grepped `build_time.rs`, but it is in `comptime_run.rs:642`.
   - How to see the generated Rust (`nikaia lower FILE` writes `FILE.rs`). I scanned `--help` to find that.
   - That CI's state was not red: the input file said `RED`, but that meant BT's gate, not CI.
   - The current list of other agents' uncommitted files. `git status` gave it, but only after a long listing.

3. **Checks:**
   - Useful: CI runs through `gh api`, the probe rows a–e on today's binary, lowering the probe to see the moving `match`, tracing `lent_coalesces` into `check_core_ops_coalesce` / `check_core_ops_behind_a_view`, a duplicate-issue search, GR's pre-flight on `Wall`, and `git diff` on `30-nikaia-tooling.md`.
   - Partly wasted: the full `ls docs/briefs` (very long), printing the whole b8 brief when only its layout mattered, the `--help` grep, and a `git diff` whose output was too large to show.

4. **The one change:** a report's `Found:` must paste the probe inline (at most 15 lines), or commit it under `docs/briefs/probes/`, never cite a scratchpad path. That rule belongs in `docs/architect.md` (*The report*) and `.claude/agents/builder.md`. Second best: a "probe skeleton" entry in builder-facts showing a map or list of structs, a `T?` field and `nikaia lower`. That would have saved most of the syntax round-trips.
