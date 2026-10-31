Result: done
Batch: y28-1085-views-back-through-if-match.md, y21-events.md, y27-load.md
Owner: none
Found: y21 run 3's NK2302 filed as #1085 (`ready`): a list or struct of views handed back through an `if`/`match` (or a block tail) is refused; handed back directly it passes. The builder's probe passed on today's binary because its struct was built from a literal in `main` (field owned); reproduced with `N` built from a `trim()` inside a function. Cause: `views.nika` `handed_back` follows only `StructLit`/`ListLit`, not `If`/`Match`/`Block` tails.
Checked: with `walk` rewritten in scratch to push into a local, the draft 1c1199d6 checks, builds, and `tests/events.nika` blesses to the suite's form; so #1085 is the only defect left before y21.
Briefs: y28 new (fix, `crates/`, version raise for the supervisor); y21 re-queued (run-3 report and debrief renamed `.run3.`, a *Run 4* paragraph with an inline pre-flight); y27's pre-flight names y21 run 4 and #1085.
Process: `docs/architect.md` (a view-checker probe must make the field really a view; check the whole draft before re-queueing), `docs/builder-facts.md` (narrow a view refusal with `return x` / `let r = x; return r`).
Issues: #1085 filed (`ready`); #928 *To build* step 4 updated (keeps `ready`, `wip`).
Pushed: see commit
