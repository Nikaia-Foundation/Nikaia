use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow};

use crate::ast::{Expr, Item};

use crate::contracts::ty::TyOps;

use crate::emit::{borrowing_structs_beside, holds_view, package_borrowing};

use crate::parser::Parsed;

/// **The ledger's small records are declared in Nikaia**
/// ([ADR-294](../../../docs/specification/adr/adr-294.md) step (b)):
/// `nikaia-std/src/tools/ty.nika`, beside the type language their fields
/// hold. Re-exported here so that every reader keeps its path.
pub use nikaia_std::tools::ty::{
    ConfigContract, Crosses, FieldContract, FnContract, Ledger, Lock, Notes, Provenance, Signature,
    Sync, Threads, TypeContract, VariantContract,
};

impl LedgerOps for Ledger {
    fn infer_package_noted(
        units: &[&Parsed],
        library: &Ledger,
    ) -> (Self, Vec<crate::check::Checked>, sync::Noted) {
        let mut ledger = Ledger {
            version: VERSION,
            toolchain: toolchain(),
            inference: INFERENCE.to_string(),
            ..Ledger::blank()
        };

        // **The item loop, in Nikaia** (`tools/ledger_items.nika`, ADR-294,
        // #125): the entries every declaration makes, and the defaults it
        // could not write as a literal (ADR-318 D1), evaluated once the
        // entries exist. What holds a view is computed per unit here, by name
        // (#544): a `Symbol` is interned by the parse of one file, so a set of
        // them means nothing to another.
        let views: Vec<nikaia_std::tools::parsed_view::ParsedView<'_, '_, '_, '_, '_, '_>> = units
            .iter()
            .map(|parsed| crate::check::parsed_view_of(parsed))
            .collect();
        let package_borrowing = package_borrowing(units);
        let borrowing: Vec<BTreeSet<String>> = units
            .iter()
            .map(|parsed| {
                borrowing_structs_beside(parsed, &package_borrowing)
                    .iter()
                    .map(|s| parsed.text(*s).to_string())
                    .collect()
            })
            .collect();
        let defaults =
            nikaia_std::tools::ledger_items::declare_items(&views, &borrowing, &mut ledger);

        // **Every unit is checked against the whole package's declarations**,
        // which is the other half of D2: a call to the file next door resolves
        // here too, so the method calls the walks below read are the package's.
        // With the other files **beside** it, so a type another file
        // declares has its fields and variants here as there (#125).
        let checked: Vec<crate::check::Checked> = units
            .iter()
            .enumerate()
            .map(|(at, parsed)| {
                let beside: Vec<&Parsed> = units
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != at)
                    .map(|(_, other)| *other)
                    .collect();
                crate::proofs::in_unit(at, || {
                    crate::check::check_for_the_ledger(
                        parsed,
                        &beside,
                        &ledger,
                        library,
                        &crate::assets::Reads::none(),
                    )
                })
            })
            .collect();
        let resolved: BTreeMap<String, crate::check::MethodCalls> = checked
            .iter()
            .flat_map(|c| c.methods.iter().map(|(k, v)| (k.clone(), v.clone())))
            .collect();

        let mut noted = sync::infer(&mut ledger, units, library, &resolved);
        // **An expression function's `ensures`** (ADR-314 D3): after `sync`,
        // which it is only given for.
        expression_ensures(&mut ledger, units);
        // Kap 7.1: `throws` in the source says *that* it fails; this says with
        // what (ADR-023 D1). After `sync`, because both read bodies and only
        // this one needs nothing from the other - and both are handed the same
        // `resolved`, because ADR-288's whole point is that there is one
        // answer to what `a.add(v)` goes to and both walks read it.
        let stored: BTreeMap<String, crate::check::StoredCode> = checked
            .iter()
            .flat_map(|c| c.stored_code.iter().map(|(k, v)| (k.clone(), v.clone())))
            .fold(BTreeMap::new(), |mut all, (key, code)| {
                let entry: &mut crate::check::StoredCode = all
                    .entry(key)
                    .or_insert_with(crate::check::StoredCode::empty);
                entry.resolved.extend(code.resolved);
                entry.unresolved |= code.unresolved;
                entry.free.extend(code.free);
                all
            });
        throws::infer(&mut ledger, units, library, &resolved, &stored);
        // **The fourth derived column** ([ADR-288](../../../docs/specification/adr/adr-288.md)
        // D2), and the one that was specified without an inference. After
        // `throws` for no reason but tidiness: it reads the same bodies through
        // the same walk and needs nothing either of the two produced.
        touch::infer(&mut ledger, units, library, &resolved);
        // **The fifth derived column** ([ADR-094](../../../docs/specification/adr/adr-094.md)
        // D2). After the item loop, because its fixpoint reads a callee's
        // `signature` — which the item loop wrote — and a callee's own
        // `keeps`, which is itself;
        // nothing above produces anything it needs, and nothing above reads
        // what it writes. **What each result points into is written first**
        // (#697): a call whose result points into an argument keeps it only
        // where the result leaves, and the walk asks the callee's `ref(..)`.
        write_result_sources(&mut ledger, units, library);
        let mut column = keeps::infer(&mut ledger, units, library, &resolved);
        // **And again, until `keeps` settles** (#720): a parameter `keeps`
        // says is kept is moved in, not viewed, so the result no longer points
        // into it, and a caller that hands its own parameter there keeps that
        // too. The first pass knew no keeps and read every plain parameter as
        // a view; `keeps` itself cannot tell a result that views a kept
        // parameter (`tag(s) -> Tag`) from one that owns it beside a literal.
        for _ in 0..8 {
            let before: Vec<Vec<String>> =
                ledger.functions.values().map(|c| c.keeps.clone()).collect();
            write_result_sources(&mut ledger, units, library);
            column = keeps::infer(&mut ledger, units, library, &resolved);
            if ledger
                .functions
                .values()
                .map(|c| &c.keeps)
                .eq(before.iter())
            {
                break;
            }
        }
        // ADR-037 D7: which count each `Shared` class gets. After `keeps`,
        // because a lambda handed to a parameter the callee keeps is a kept one
        // (#532); and it resolves a callee's parameters against the
        // `signature` step 1 wrote and a type's parts against its `fields`.
        //
        // **A unit at a time, against the package's ledger.** It summarises the
        // `Shared` values a body holds rather than folding a call graph, so
        // there is no fixpoint to run across units - what it needed from its
        // neighbours is the callee's `signature`, and that is in the ledger the
        // loop above built.
        for parsed in units.iter().copied() {
            sharing::infer(&mut ledger, parsed, library);
        }
        // **The `maps` key** (ADR-010 D9, #581 step 6): read from the bodies
        // of the package's `pub` functions, and written as a permission.
        maps::infer(&mut ledger, units);
        // Beside `keeps`, and for the same reason it runs here: it reads the
        // checker's method answers and the entries the item loop wrote.
        locks::infer(&mut ledger, units, library, &resolved, &stored);
        // **Last, and it reads none of the columns above**
        // ([ADR-283](../../../docs/specification/adr/adr-283.md) D5): what it
        // asks is about a signature's shape and about which buffer a returned
        // view came from, and no inference above answers either. It is also the
        // one that changes no lowering — the state it writes is a
        // representation and only one of the three is built.
        tether::infer(&mut ledger, units, library);
        // **What a result points into, written** (ADR-283 D25, #580): the
        // declaration listed every view, the body knows which one. After
        // `keeps`, which says which plain parameters are lowered as views, and
        // twice, so that a callee of another unit is read as written once its
        // unit has been (the first pass reads the declaration's list, which
        // only ever says more).
        write_result_sources(&mut ledger, units, library);
        evaluate_defaults(&mut ledger, units, &defaults, library);
        attribute_values(&mut ledger, units, library);
        noted.column = column;
        (ledger, checked, noted)
    }
}
