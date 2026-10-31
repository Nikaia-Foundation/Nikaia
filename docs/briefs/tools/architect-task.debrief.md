1. **Most turns:** finding which AST variant `id[i64](3)` parses to, and which walk hands it to `sync.nika` as `Seen::Opaque`. The search went `where.nika` → `grep Opaque` in `sync.nika` → `enum Seen` → `foreign.nika` → counting `Expr::` variants → `check_core_call.nika`. That was about 6 turns of grepping for something one line in builder-facts could say. Rebuilding #1019's body through Python and `sed` to strip the footer also cost a turn or two.
2. **Could have been handed to me:**
   - That an explicit generic call is `Expr::Index { base, .. }`, resolved by `check_core_call_filled`.
   - Where `enum Seen` and the effect walk live (`foreign.nika`).
   - That `docs/briefs/tools/architect-input.txt` was the CUT line, while `qa-input.txt` and `unpushed.txt` were stale leftovers from an earlier round. I read them and briefly took them for the current state.
   - That local was 8 commits behind, with an untested builder commit (417abcf) on top. That made the push rejection predictable.
3. **Checks run:**
   - Useful: three probes on today's binary. They confirmed #1095's minimal form, j7's second probe and #1096. The search for duplicate issues and the `where.nika` lookup also earned their place.
   - Partly wasted: the first `turn[Contact]` probe also tripped NK1104 (it returned `f.name` without `.clone()`), so I re-ran it. The push attempt was predictable: 417abcf is unpushed and untested, so it could never go out from this checkout.
   - Not run: I never checked which `std` call the checker counts as pausing for j7's second test, and left that to the builder.
4. **One change:** have the driver write the git state (ahead/behind, unpushed shas) and which input files are fresh into `architect-input.txt`. Separately, give builder-facts a short entry on how each call form is represented as an `Expr`, and which walks name a callee (`a_named_call`, `check_core_call`). Five of my lookups would have gone, and so would the j5 builder's guess that a bracket call can't be resolved.
