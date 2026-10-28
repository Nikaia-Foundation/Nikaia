Result: done
Files: crates/nikaia/src/modules.rs crates/nikaia/src/contracts/sync.rs crates/nikaia-std/src/tools/modules.nika crates/nikaia-std/src/tools/lowered/modules.rs crates/nikaia-std/src/tools/lowered.rs scripts/self_hosting.py docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: modules: the program assembly into Nikaia (M1)
Body: Program::of's dependency pass (described crates into library, each dependency group's ledger absorbed as promised into library and as it is into contracts, then each package's view of what it reaches, #497) is dependency_ledgers in modules.nika; package_ledger is its callback, called inside the Nikaia loop so library grows group by group; the own groups' loop stays Rust and absorbs through absorb_own. declared_once, groups, as_promised, contracts_as_promised, lost_sync (render_finding as callback), shown_paths, package_names, is_single_file moved; Pause is modules.nika's ModulesPause (Site from sync.nika), re-exported in contracts::sync. could_promise and of_a_dependency now take the as_its_own map rather than a key set. Built from the partial diff of the previous M1 run, cut by the driver's 30-minute limit; its loops in shown_paths and package_names put back to the draft's chains (they compile; Seq has no enumerate, so shown_paths maps a range). Plan row M1 marked built.
Part: #125
Pushed: 3136e7b77a9acfab2e6b46d5fd98f27b59dc2c91
Tests: it filtered on package depend promise sync use_ declared test plus guard set 248 passed; nikaia test --both-settings tests/compiler 203 passed; lower-std fixpoint (second run unchanged); clippy clean; self-hosting 80.1 %
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: kept 15 of 118 lines (draft against the lines the move added to modules.nika); costliest: 1) Program methods and Program::of drafted as an impl over Rust's Program/Unit (the brief keeps them Rust), so every signature and self.units access rewritten to free functions over packages/renames/paths and a package_ledger callback; 2) R58 `// ADAPTER: bleibt in Rust (Ledger)` on every Ledger function although Ledger is ty.nika's, and as_promised drafted with map iter_mut (no tools file uses it) rewritten as collect-then-insert; 3) Option matches drafted as `match x { v => .., null => {} }` over split_once/get/filter (lost_sync, the reaches loop) rewritten as `?? continue`, and `p.clone()` on a String? (NK1125) needing `p?.clone()`
