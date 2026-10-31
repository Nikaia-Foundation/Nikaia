Result: done
Files: crates/nikaia-std/src/tools/ledger_items.nika crates/nikaia-std/src/tools/check_attributes.nika crates/nikaia-std/src/tools/lowered/ledger_items.rs crates/nikaia-std/src/tools/lowered/check_attributes.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/contracts/mod.rs crates/nikaia/src/check/mod.rs crates/nikaia/src/check/attributes.rs docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: contracts: the declared-items loop, computed defaults and attribute values move into ledger_items.nika (C1a)
Body: `declare_items`, `ComputedAt`, `computed_defaults`, `attribute_values` (plus the private `attribute_texts`) are in `ledger_items.nika`. I started from the first run's step-1 patch and swapped in ADR-384's `or_default()` and `.cloned()`. `infer_package_noted` now builds the `ParsedView`s and the per-unit `borrowing` name sets in Rust and calls `declare_items`. `evaluate_defaults` finds each default and its type by index through the new `computed_default_at`. `attribute_values` hands Nikaia a `computed(unit, ty, argument)` callback over `computed_literal`. Nothing calls `trait_method`, `LedgerOps::function`, `check::attributes::{MARK, mark_text}` any more, so they are deleted, which also removes the then-empty `check/attributes.rs` and its `mod` line in `check/mod.rs` (clippy -D warnings would refuse the dead code). `check_attributes_mark_name` is now `pub`. Plan row C1 updated: C1a built; C1b = the `stored` fold, `lookup`, `candidates`. Self-hosting share 82.6 %.
Part: #125
Tests: it `ledger contract default attribute trait impl extern grammar` 337 passed; `--lib contracts::` 36 passed; tests/compiler programs named default/attribute/ledger/trait/extern/mark (15) pass with --both-settings in a scratch copy, and the nikaia.contracts/derived/proofs written there match tests/compiler's byte for byte. Ledger probe (attributes on type, field and variant, the mark, computed defaults on a free fn, a method and a struct field, trait, impl for): nikaia.contracts and nikaia.derived identical between the suite worktree's binary and mine. lower-std fixpoint: only ledger_items.rs, check_attributes.rs and lowered.rs changed (#1060 guard clean). clippy 1.97 and 1.99.1, fmt, pre-push-check clean.
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0 (target/nikaia-project-tests 3.8G, / 5.7G free; new entries over 100 MB: build-time-parsers/768d2772 1163 MB, run-bundle/3f8b93b8 287 MB, rlib/af187e66 266 MB; deleted /home/user/nikaia-clippy-new)
Rs2nika: kept 23 of 99 lines (under 60%). Costliest edits: the draft's whole `infer_package_noted` body around the loop deleted as the brief asked; `Computed<'a>` with `&Expr`/`&Type` redone as the index-based `ComputedAt`, so each push site was rewritten; the `values` closure and the `At` enum redone as `attribute_texts` and the `(kind, place)` pair
Pushed: local 398da2fada1d0129c3bac2eb0b7c7afc459bf276
