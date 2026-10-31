1. **Most turns:** finding why `[1, 2]` has no element type. About 8 greps went into tracing it: `ListLit` led to `check_core_list_literal`, then `LitInt` turned out to be `Unknown`, and the cause was finally the open-number machinery (ADR-285 D34) in `check_numbers3.nika` and `check_core_stmt.nika:720-770, 1560`. The brief's hint ("integer-literal or not-yet-defaulted type, or `Ty::Unknown`") was right but named no place to look.
2. **Looked up, could have been handed:**
   - The fact that integer literals are `Ty::Unknown` until each function's open numbers are decided after its body.
   - The existing `open_elements` branch in the `for` checker, about 50 lines below the spot the brief pointed to; the fix belongs there.
   - How `num::value` is defined (the `Value` trait), which told me the nested rows can't work with a copy alone.
3. **Checks run:**
   - One `lower-std`/build cycle.
   - The old and new binary on the probe, plus an edge probe (`mut` list, `_`, printing).
   - A standalone rustc copy of `num::value` for the nested case.
   - `tests/compiler --both-settings`, the guard set, clippy and `pre-push-check`.

   None was wasted. The rustc copy saved a build: it showed the nested rows fail for a second reason.
4. **One change:** where the probe matrix shows a declared-versus-untyped split, the brief should name the inference pass that types untyped values. Here that is ADR-285 D34 open numbers: `flow.open_numbers` and `open_elements`, and their one-level `element` flag. The architect could also have spotted in advance that the nested rows need D34 one level deeper and split them into their own step.

Process note: `Disk freed` was filled in after `commit.nika` ran. The report file is not among the committed files, so nothing committed holds the `DISK` placeholder.
