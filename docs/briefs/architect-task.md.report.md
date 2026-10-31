Result: done
Batch: c1a-ledger-items.md, p5c-project-drive.md
Owner: none

- K3b blocked: filed #1077 (tools preamble lacks DurationExt/DurationIn/InstantSince) and #1078 (filter_map(split_once).map(fn((a,b))) lowers parts as &str), both `ready` + `wip`; briefs b17, b18 appended to bugs/batch.txt with `Blocks: k3b-comptime-bundle.md`. Third Found (project_members drift) already fixed by 84fbfb99.
- New move C1a (contracts/mod.rs declare_items/attribute_values into ledger_items.nika); P5c re-cut at 4520d400, its K3b dependency forwarded to the Rust write_if_changed.
- Held back: K3b (b17, b18), GR (b15), S1 (after b17, shares sysroot.rs), P5e (O rows).
- Pushed 7ae15781, merge f96bb09a (restored the y21 rule a scratch-index commit dropped), 7233faf8 (push-as-merge rule in docs/architect.md). dcc656de is an empty commit from a failed edit step, harmless.
- Supervisor owes: s922b version raise + Changelog line; `rustup update stable` before clippy.
