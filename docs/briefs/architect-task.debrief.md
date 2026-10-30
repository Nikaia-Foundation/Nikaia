1. **What cost the most:** making the Found reproducible. Before I could write #1007 I had to learn how `sum` is lowered today. That took three greps (`std.contracts`, the `collect` turbofish in `emit_core_calls.nika`, and #931's commit), plus two probe rounds, one for `i64` and one for `f64`. Reading the 434-line `docs/architect.md` cost the most context, since its *State* section alone is about 110 lines.
2. **What the brief could have handed me:**
   - That `k3a2-comptime-compute.md` was already re-queued and its first report already renamed `.run1.report.md`. I checked this with `ls`.
   - Where a method-call turbofish is written (`emit_core_calls.nika`, about line 1183) and that #931 (5162370e) is the related fix. I found both by grep.
   - That `common.part` has a `<full brief path>` placeholder to fill in. I confirmed it by reading the file.
3. **Checks:**
   - Useful: the four probes on `target/fast/nikaia` (they fixed the issue's scope: bare `println`, a `let` with no type, inside `+`; `count` and `max` are fine), the `f64` probe (it confirmed the planned test row), and `git status -sb` after the push.
   - Wasted: the `nikaia lower` call. It only showed `.copied().sum()`, which the error message had already implied.
   - Probably extra: tailing k3a2 to look for its Attribution block.
4. **The change that would save the most:** `architect-task.md` (or `architect-input.txt`) should name the cut brief's report and debrief paths, and say whether the remaining briefs are still queued and their preconditions met. Separately, `docs/architect.md` should keep only the last 5 *State* entries and move older ones to `docs/architect-history.md`. That would roughly halve the startup read.
