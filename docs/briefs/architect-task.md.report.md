Result: done
Batch: gr2-grammar-run-reland.md s1082-or-default-cloned.md c1a-ledger-items.md
Owner: none
Decided: GR's `needs architect decision` (trust per package or per file) is already answered by ADR-010 D9: per class after the whole package, per file and a toolchain exemption both rejected. `process::run` is untrusted in `std.contracts`, the first untrusted source in `tools`; `project_members.rs`'s fs::Path keys come through a trait (a D9.5 barrier), so its maps become keyed. The brief now has that drift committed with the move, starting from the saved resolution `gr2-grammar-run-reland.wip.patch`.
Filed: nothing new (b19 done, no Found). To check later (in State): `trust.rs` `join` makes everything fast when no untrusted source is read, even for keys from a trait barrier.
