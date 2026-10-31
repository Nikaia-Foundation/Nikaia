1. **Most turns:** finding where the checker types a `Seq` method. I grepped `"filter_map"` and `"map"` through the `check_*.nika` files and the Rust adapter, about 6 turns, before finding the signature in `std.contracts`. The brief's guess for row a (a `text_tiers` branch) was wrong: the cause was the checker's type binding, spread over three spots in two files. The 10-minute `tests/compiler` run took the most time.
2. **What the brief could have handed me:**
   - `Seq` methods are typed from their `signature` in `crates/nikaia-std/std.contracts`.
   - Signature types are parsed by `ty::parse` and generics matched by `ty::bind`.
   - A lambda argument's type comes from `check_core_args_lambda`.

   One line under *Where things live* ("std method typing: `std.contracts` signature → `ty.nika` parse/bind/substitute → `check_core_args.nika`") would have saved most of the search.
3. **Checks run:**
   - Useful:
     - probes on the unfixed suite binary and on mine;
     - two build cycles with the fixpoint check, which caught the `project_settings.rs` re-lowering;
     - the `tests/compiler` run, the `it` subset, the `nikaia-std` unit tests, fmt, clippy and `pre-push-check.sh`.
   - Wasted:
     - a hand-written wait loop that matched itself through `pgrep` and ran until its time limit;
     - one extra build to confirm the fixpoint;
     - the probes on named functions and `if`/`null` arms, which only found unrelated bugs.
   - Missed: rebuilding without the `text_tiers` change, so I never showed whether that change is needed. My first commit message claimed it wasn't needed, unchecked, and I had to amend it.
4. **One change:** the brief's *Stop if* rule allowed edits only in `text_tiers` plus "the checker file of row b", but the cause spanned `ty.nika` and `check_core_args.nika`. Allow "the checker's type files (`ty.nika`, `check_core_*.nika`)". Also add to builder.md: "a test's `.stdout` list has no name filter; `nikaia test` always runs the whole project", so briefs stop asking for tests "by name".
