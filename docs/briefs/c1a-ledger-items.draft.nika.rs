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

        // **The package's types and not the file's.** `impl_parameters` asks
        // whether a name in `impl Box[Thing]` is a type or a type *parameter*,
        // and a `Thing` declared in the file next door is a type.
        let declared: BTreeSet<String> = units.iter().flat_map(|u| declared_types(u)).collect();

        // The options whose default is computed rather than written as a
        // literal (ADR-318 D1), evaluated once the entries exist.
        let mut defaults: Vec<Computed<'_>> = Vec::new();
        // What holds a view anywhere in the package, by name (#544).
        let package_borrowing = package_borrowing(units);
        for parsed in units.iter().copied() {
            // Per unit, and it has to be: a `Symbol` is interned by the parse
            // of one file, so a set of them means nothing to another.
            let borrowing = borrowing_structs_beside(parsed, &package_borrowing);
            let borrowing_names: BTreeSet<String> = borrowing
                .iter()
                .map(|s| parsed.text(*s).to_string())
                .collect();

            for item in &parsed.program.items {
                match &item.node {
                    Item::Fn { .. } => {
                        let (name, contract) =
                            ledger.function(parsed, &item.node, None, &BTreeSet::new());
                        computed_defaults(&mut defaults, &name, parsed, &item.node);
                        ledger.functions.insert(name, contract);
                    }
                    Item::Impl {
                        target, methods, ..
                    } => {
                        // `impl Stack[T]` puts `T` in scope for every method in it,
                        // so it is a name that stands for a type there too.
                        let outer: BTreeSet<String> = impl_parameters(parsed, target, &declared)
                            .into_iter()
                            .collect();
                        let target = parsed.text(target.name).to_string();
                        // ADR-295 D15: `impl Speaks for Dog` is the claim that a
                        // `Dog` may stand where a `Speaks` is asked for, and it
                        // is the only place that claim is made.
                        if let Item::Impl {
                            trait_name: Some(trait_name),
                            ..
                        } = &item.node
                        {
                            ledger
                                .implementations
                                .entry(parsed.text(*trait_name).to_string())
                                .or_default()
                                .insert(target.clone());
                        }
                        for method in methods {
                            let (name, contract) =
                                ledger.function(parsed, &method.node, Some(&target), &outer);
                            computed_defaults(&mut defaults, &name, parsed, &method.node);
                            ledger.functions.insert(name, contract);
                        }
                    }
                    // Kap 4.7: a trait's methods are recorded under the trait's own
                    // name - `Summarize::summary` - which is what lets a bound be
                    // looked up ([ADR-295](../../../docs/specification/adr/adr-295.md)
                    // D3). The same key shape an `impl`'s methods get, because a
                    // bound and a receiver ask the same question: what does a value
                    // of this thing have.
                    // **An `extern "C"` declaration is an entry like any
                    // other, and reads unlike a trait method**
                    // ([ADR-302](../../../docs/specification/adr/adr-302.md)
                    // D2). Two things are turned around, and only one of them
                    // by this record. It is **`sync`**, asserted rather than
                    // inferred, which is the shape `std`'s own hand-written
                    // entries have for the same reason: the body is in another
                    // language and this compiler does not read it. C has no
                    // suspension point at all, and a C function that sleeps
                    // *blocks* — `println`'s question (ADR-288 D21) and not this
                    // one. And it carries **no `throws`**, which is what a
                    // body-less declaration carries anyway: C has no failure
                    // channel this language reads.
                    //
                    // `touches` and `locks` are absent, which is D4 and is
                    // fail-closed: absent `touches` reads as *touches
                    // everything* (ADR-292) and absent `locks` is that column's
                    // third answer. A C signature says **less** than a Rust
                    // one, not more.
                    Item::Extern { declarations, .. } => {
                        for declared in nikaia_std::tools::ledger_items::declared_extern(
                            &parsed.interner,
                            &parsed.aliases,
                            declarations,
                        ) {
                            ledger.functions.insert(declared.key, declared.contract);
                        }
                    }
                    Item::Trait {
                        name,
                        methods,
                        is_public,
                    } => {
                        let own = parsed.text(*name).to_string();
                        for method in methods {
                            let (key, contract) =
                                trait_method(parsed, &own, &method.node, *is_public);
                            ledger.functions.insert(key, contract);
                        }
                        ledger.traits.insert(
                            own,
                            methods
                                .iter()
                                .map(|m| parsed.text(m.node.name).to_string())
                                .collect(),
                        );
                    }
                    // **An `enum` is a type a consumer has to know the cases of**
                    // (Part I 3.4: a `match` handles every possible case), and
                    // until this was here it had no entry at all — so a `match`
                    // over a dependency's `enum` could not be shown total and
                    // `NK1151` asked for an `else` that makes *a type gaining a
                    // variant* silent forever after.
                    Item::Enum {
                        name,
                        variants,
                        is_public,
                        ..
                    } => {
                        // **In Nikaia** (`tools/declared.nika`, ADR-294, #125).
                        ledger.types.insert(
                            parsed.text(*name).to_string(),
                            nikaia_std::tools::declared::declared_enum(
                                &parsed.interner,
                                &parsed.aliases,
                                variants,
                                *is_public,
                                &borrowing_names,
                            ),
                        );
                    }
                    Item::Struct {
                        name,
                        generics,
                        fields,
                        is_public,
                        is_extern,
                        ..
                    } => {
                        let tethered = nikaia_std::tools::declared::tethered_fields(
                            &parsed.interner,
                            fields,
                            &borrowing_names,
                        );
                        let key = parsed.text(*name).to_string();
                        let mut field_types = nikaia_std::tools::declared::declared_fields(
                            &parsed.interner,
                            &parsed.aliases,
                            generics,
                            fields,
                            *is_extern,
                        );
                        for (at, f) in fields.iter().enumerate() {
                            // A literal is its own text; anything else is
                            // computed with the options' defaults below
                            // (ADR-331 D5, ADR-318 D1-D3).
                            match &f.default {
                                Some(d) if a_literal(&d.value) => {
                                    field_types[at].default =
                                        nikaia_std::tools::declared::default_text(&d.value);
                                }
                                Some(d) => defaults.push(Computed {
                                    key: key.clone(),
                                    at,
                                    parsed,
                                    default: &d.value,
                                    option: parsed.text(f.name).to_string(),
                                    ty: &f.ty,
                                    field: true,
                                }),
                                None => {}
                            }
                        }
                        ledger.types.insert(
                            parsed.text(*name).to_string(),
                            TypeContract {
                                public: *is_public,
                                fields: field_types,
                                variants: Vec::new(),
                                // Never inferred: a `struct` declared here records
                                // its fields, and `contracts::send` walks those.
                                // The key exists for types whose parts are Rust.
                                crosses: Crosses::Undecided,
                                // Nothing a `.nika` file declares iterates at all
                                // yet, let alone fallibly: the types that do are
                                // `std`'s, and `std` writes them down (ADR-025 D6).
                                iterates_fallibly: false,
                                // A declared type's own parts decide whether it
                                // compares, so nothing is written here: this
                                // column is for a type whose parts are Rust.
                                compares: false,
                                orders: false,
                                equates: false,
                                copies: false,
                                // Nor does a declared `struct` read anything when
                                // it is read: a field access is memory. The types
                                // that are not are `std`'s, whose bodies are Rust
                                // ([ADR-281](../../../../docs/specification/adr/adr-281.md) D34).
                                touches: Vec::new(),
                                tethered,
                                constant: String::new(),
                                // **Its `@meta::Attribute(…)`, where it is an
                                // attribute** (ADR-331 D8), so a package that
                                // uses it reads it as this one does.
                                mark: item
                                    .attributes
                                    .iter()
                                    .find(|a| {
                                        parsed.unaliased(parsed.text(a.name))
                                            == crate::check::attributes::MARK
                                    })
                                    .map(|a| crate::check::attributes::mark_text(parsed, a))
                                    .unwrap_or_default(),
                                attributes: Vec::new(),
                            },
                        );
                    }
                    // **Every `entry` rule of a grammar is an entry**
                    // ([ADR-296](../../../docs/specification/adr/adr-296.md) D25).
                    // A grammar is entered by an ordinary call — `Json.value(input)`
                    // — so the thing entered has to be an ordinary contract, and
                    // the one column it must carry is `throws`: a rule past a
                    // commit point can fail ([ADR-023](../../../docs/specification/adr/adr-023.md)
                    // D9), and a `catch` beside the entry would meet `NK1134`
                    // without it.
                    //
                    // **`["ParseError"]` since
                    // [ADR-296](../../../docs/specification/adr/adr-296.md) D38.**
                    // It used to be `["?"]` — *something this compiler cannot
                    // name* — because a parse fails with a **rendered string**
                    // and a string is not a type. It was the last `"?"` in the
                    // tree, and seven of the corpus' eight `main`s carried it
                    // into their own set; a type is all it ever needed.
                    Item::Grammar(def) => {
                        for declared in nikaia_std::tools::ledger_items::declared_grammar(
                            &parsed.interner,
                            &parsed.aliases,
                            def,
                        ) {
                            ledger.functions.insert(declared.key, declared.contract);
                        }
                    }
                    _ => {}
                }
            }
        }

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

/// **An option's default that is not a literal** (ADR-318 D1): the function's
/// key, the option's place in its list, the unit and the expression.
struct Computed<'a> {
    key: String,
    at: usize,
    parsed: &'a Parsed,
    default: &'a Expr,
    /// The option's name and declared type, which a compiled run needs.
    option: String,
    ty: &'a crate::ast::Type,
    /// A struct's field rather than a function's option (ADR-331 D5): `key`
    /// is the type and `at` the field's place.
    field: bool,
}

/// The options of `item` whose default is an expression to evaluate.
fn computed_defaults<'a>(
    into: &mut Vec<Computed<'a>>,
    key: &str,
    parsed: &'a Parsed,
    item: &'a Item,
) {
    let Item::Fn { config, .. } = item else {
        return;
    };
    for at in nikaia_std::tools::ledger_defaults::computed_options(item) {
        let option = &config[at as usize];
        into.push(Computed {
            key: key.to_string(),
            at: at as usize,
            parsed,
            default: &option.default,
            option: parsed.text(option.name).to_string(),
            ty: &option.ty,
            field: false,
        });
    }
}

/// **A field's attributes, as the values they come to** (ADR-331 D6, D8):
/// each a struct literal of literals, every field of the attribute's struct
/// given - from the argument, or from its default. An attribute whose
/// arguments are not literals or variants is the checker's to refuse, and is
/// left out here.
fn attribute_values(ledger: &mut Ledger, units: &[&Parsed], library: &Ledger) {
    // Where each value goes: the type itself, a field or a variant.
    enum At {
        Type,
        Field(usize),
        Variant(usize),
    }
    let mut found: Vec<(String, At, String)> = Vec::new();
    let values = |parsed: &Parsed, attributes: &[crate::ast::Attribute]| -> Vec<String> {
        attributes
            .iter()
            .filter_map(|attribute| {
                let named = parsed.unaliased(parsed.text(attribute.name));
                if named == crate::check::attributes::MARK {
                    return None;
                }
                // In Nikaia (`tools/ledger_defaults.nika`, ADR-294, #125); the
                // workshop that computes a non-literal argument stays here.
                nikaia_std::tools::ledger_defaults::attribute_value(
                    ledger,
                    library,
                    &parsed.interner,
                    &named,
                    attribute,
                    &|ty: &ty::Ty, argument: &Expr| {
                        computed_literal(
                            units,
                            parsed,
                            "an attribute's argument",
                            argument,
                            ty,
                            library,
                        )
                    },
                )
            })
            .collect()
    };
    for parsed in units.iter().copied() {
        for item in &parsed.program.items {
            let owner = match &item.node {
                Item::Struct { name, .. } | Item::Enum { name, .. } => {
                    parsed.text(*name).to_string()
                }
                _ => continue,
            };
            for text in values(parsed, &item.attributes) {
                found.push((owner.clone(), At::Type, text));
            }
            match &item.node {
                Item::Struct { fields, .. } => {
                    for (at, field) in fields.iter().enumerate() {
                        for text in values(parsed, &field.attributes) {
                            found.push((owner.clone(), At::Field(at), text));
                        }
                    }
                }
                Item::Enum { variants, .. } => {
                    for (at, variant) in variants.iter().enumerate() {
                        for text in values(parsed, &variant.attributes) {
                            found.push((owner.clone(), At::Variant(at), text));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for (owner, at, text) in found {
        let (kind, place) = match at {
            At::Type => (0, 0),
            At::Field(at) => (1, at as i64),
            At::Variant(at) => (2, at as i64),
        };
        nikaia_std::tools::ledger_defaults::apply_attribute(ledger, &owner, kind, place, text);
    }
}
