use crate::contracts::LedgerOps;

use std::collections::{BTreeMap, BTreeSet};

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::ast::Item;

use crate::parser::{self, Parsed};

use nikaia_std::error::Thrown;

use nikaia_std::rt::exec::block_on;

use nikaia_std::tools::modules::{
    self as collecting, ModulesHost, ModulesLedgerHost, ModulesRefusal, ModulesUnit,
};

use std::cell::RefCell;

/// A package this program depends on: the name a `use` writes, and where it is
/// ([ADR-286](../../../docs/specification/adr/adr-286.md) D15). Declared in
/// `tools/project_members.nika`, which finds them (P4a).
pub use nikaia_std::tools::project_members::ProjectMembersDependency as Dependency;

impl Program {
    fn of(
        mut units: Vec<Unit>,
        dependencies: &[Dependency],
        described: &crate::contracts::Ledger,
    ) -> Result<Program> {
        // **A `test` block is compiled only by `nikaia test`** (ADR-269 D1),
        // which has turned every one it runs into a function by now: what is
        // left is left out.
        for unit in &mut units {
            unit.parsed
                .program
                .items
                .retain(|item| !matches!(item.node, Item::Test { .. }));
        }
        // **A type's rule is a fact only where no file of its package, and
        // no package using it, can break it** (ADR-369 D3).
        for range in groups(&units) {
            let mut files: Vec<(&mut crate::parser::Parsed, &str)> = units[range]
                .iter_mut()
                .map(|u| (&mut u.parsed, u.source.as_str()))
                .collect();
            crate::type_rules::across_files(&mut files);
        }
        // **The dependencies first, and the program's own package last**
        // (ADR-100 D5): their ledgers, merged in Nikaia
        // (`tools/modules.nika`, ADR-294, #125).
        let mut contracts = crate::contracts::Ledger::empty();
        let mut library = crate::contracts::std_library();
        let ranges = groups(&units);
        let mut reaches = BTreeMap::new();
        let mut their_renames = BTreeMap::new();
        for dependency in dependencies.iter().rev() {
            reaches.insert(dependency.name.clone(), dependency.reachable.clone());
            their_renames.insert(dependency.name.clone(), dependency.renames.clone());
        }
        let as_its_own = nikaia_std::tools::modules::dependency_ledgers(
            &mut library,
            &mut contracts,
            described,
            &packages_of(&units),
            &units.iter().map(|u| u.renames.clone()).collect::<Vec<_>>(),
            &reaches,
            &their_renames,
            &|from: i64, to: i64| {
                let group = &units[from as usize..to as usize];
                package_ledger(
                    group,
                    dependencies
                        .iter()
                        .find(|d| Some(&d.name) == group[0].package.as_ref()),
                    // **`std` alone for a dependency's own inference.** Its own
                    // dependencies are deliberately invisible here (ADR-286 D21),
                    // which is exactly why a ledger derived on this side is the
                    // second-best answer and D1 prefers the one it shipped.
                    &crate::contracts::std_library(),
                )
            },
        );

        let mut paused_by_code = BTreeMap::new();
        let mut pauses = BTreeMap::new();
        // The library is `std`'s exactly when nothing was added to it above.
        let std_alone =
            described.functions.is_empty() && described.types.is_empty() && as_its_own.is_empty();
        let mut own_checked = None;
        let mut own_groups = 0;
        for group in ranges {
            if units[group.start].package.is_some() {
                continue;
            }
            own_groups += 1;
            let renames = units[group.start].renames.clone();
            let (own, checked, noted) =
                package_ledger_checked(&units[group.clone()], None, &library);
            // **Kept where it is the join's pass**: the one group of the
            // program's own files, all of them, against `std` alone.
            own_checked = checked.filter(|_| {
                std_alone && group.len() == units.iter().filter(|u| u.package.is_none()).count()
            });
            paused_by_code.extend(noted.by_code);
            pauses.extend(noted.why.into_iter().map(|(name, mut pause)| {
                pause.unit += group.start as i64;
                (name, pause)
            }));
            nikaia_std::tools::modules::absorb_own(&mut contracts, &renames, &own);
        }

        Ok(Program {
            units,
            entry_shown_as: None,
            described: crate::contracts::Ledger::blank(),
            contracts,
            as_its_own,
            tests: Vec::new(),
            paused_by_code,
            pauses,
            own_checked: own_checked.filter(|_| own_groups == 1),
            checked_facts: BTreeMap::new(),
        })
    }
    /// The same, told what this build may read while it builds
    /// ([ADR-310](../../../docs/specification/adr/adr-310.md)).
    ///
    /// **A second entry point and not a field on `Build`**: `Build` is the
    /// machine and the switches, copied freely, and what a build may read is a
    /// fact about the invocation with a lifetime on it. A caller with nothing
    /// to say passes [`crate::assets::Reads::none`], which is D1.
    pub fn emit_reading(
        &self,
        build: crate::emit::Build,
        reads: &crate::assets::Reads,
    ) -> Result<crate::emit::Lowered> {
        use crate::emit::{Lowered, Needs, SourceMap};

        // **The package's trust, not the entry's** (ADR-010 D9, #581): a map is
        // one type in every file, so a socket read in any file keys them all.
        let package: Vec<&crate::parser::Parsed> = self
            .units
            .iter()
            .filter(|u| u.package.is_none())
            .map(|u| &u.parsed)
            .collect();
        // Each question the package's ledger walk asks, in its file.
        let files = self
            .units
            .iter()
            .filter(|u| u.package.is_none())
            .map(|u| (u.path.clone(), u.source.clone()))
            .collect();
        let mut trust = crate::proofs::with_units(files, || match &self.own_checked {
            Some(checked) => {
                crate::contracts::trust::analyse_package_checked(&package, std_ledger(), checked)
            }
            None => crate::contracts::trust::analyse_package(&package, std_ledger()),
        });
        // **A package whose `pub` items hold maps fills them with the hasher of
        // the program that uses it** (ADR-010 D9), and the ledger says what
        // each function of it puts in (13.5, `maps`): a program that calls one
        // whose line is `keyed`, or that has no line, hands its maps to a body
        // that fills them with keys the program cannot vouch for, so it is no
        // more trusted than that. `from(..)` adds nothing the program has not
        // chosen: a trusted program's arguments are trusted, and an untrusted
        // program's maps are keyed already. The ledger is read, never the
        // body.
        let mut packages: std::collections::BTreeMap<&str, Vec<&crate::parser::Parsed>> =
            std::collections::BTreeMap::new();
        for unit in self.units.iter().filter(|u| u.package.is_some()) {
            if let Some(name) = unit.package.as_deref() {
                packages.entry(name).or_default().push(&unit.parsed);
            }
        }
        if trust.provenance == crate::contracts::Provenance::Trusted {
            let called = names_in(&package);
            'packages: for (name, files) in &packages {
                for key in crate::contracts::maps::keyed_functions(files, &self.contracts, name) {
                    let last = key.rsplit("::").next().unwrap_or(&key);
                    if called.contains(last) {
                        trust.provenance = crate::contracts::Provenance::Untrusted;
                        break 'packages;
                    }
                }
            }
        }
        // **A call to a barrier** (D9): a method of a trait that holds a map in
        // its signature, a `pub` function that hands a map to one, a function
        // of a described Rust crate that takes one. The map arrives there with
        // std's keyed hasher, so a program that calls one keeps no fast map
        // across it: its maps that leave a function are keyed.
        // And a package that declares one - a trait method with a map, a
        // function that hands a map to one, a method whose result alone holds
        // one - writes the keyed map in its own bodies where they make it.
        if trust.provenance == crate::contracts::Provenance::Trusted
            && !crate::emit::barrier_names(&package, &self.described).is_empty()
        {
            trust.provenance = crate::contracts::Provenance::Untrusted;
        }
        if trust.provenance == crate::contracts::Provenance::Trusted {
            let all: Vec<&crate::parser::Parsed> = self.units.iter().map(|u| &u.parsed).collect();
            let barriers = crate::emit::barrier_names(&all, &self.described);
            if !barriers.is_empty() && names_in(&package).iter().any(|n| barriers.contains(n)) {
                trust.provenance = crate::contracts::Provenance::Untrusted;
            }
        }
        let needs = self.units.iter().fold(Needs::none(), |acc, u| {
            acc.join(&crate::proofs::for_unit(
                &u.path,
                &u.source,
                u.package.is_some(),
                || crate::emit::needs_of(&u.parsed),
            ))
        });
        // **Every file, for every file.** What a `comptime` came to is the
        // checker's answer and the emitter writes it, so the two have to be
        // asked the same question — a unit lowered against fewer files than it
        // was checked against would refuse an item the check accepted.
        let beside: Vec<&crate::parser::Parsed> =
            self.units.iter().map(|unit| &unit.parsed).collect();

        let mut rust = String::new();
        rust.push_str("// Generated by the Nikaia bootstrap compiler (Stage 0).\n");
        rust.push_str("// Edit the .nika source, not this file.\n\n");
        rust.push_str(&needs.preamble());
        rust.push('\n');

        let mut map = SourceMap::default();
        let mut published = std::collections::BTreeMap::new();
        for (at, unit) in self.units.iter().enumerate() {
            // **A dependency's files are read and not written** (ADR-286 D19).
            // They are here so the ledger and the checks know what the package
            // offers; the crate that offers it is generated beside this one, and
            // a qualified name needs no help from us - `c::thing()` is emitted
            // as written and Rust resolves `c` to the crate the manifest names.
            //
            // The index is still `self.units`', because a diagnostic about a
            // dependency's file has to name that file.
            if unit.package.is_some() {
                continue;
            }
            let body = crate::proofs::for_unit(&unit.path, &unit.source, false, || {
                crate::emit::emit_module_body_at(
                    &unit.parsed,
                    &beside,
                    build,
                    trust.provenance,
                    &self.contracts,
                    &self.described,
                    // The entry is the only file ADR-303 D4's generated `fn main`
                    // may be written from.
                    at == 0,
                    reads,
                    self.checked_facts.get(&at).cloned(),
                )
            })
            // **A refusal from the lowering gets its line here**
            // ([ADR-171](../../../docs/specification/adr/adr-171.md) D2), which
            // is the one place that has both the byte and the file: the
            // lowering knows the statement and not the path, and whoever
            // catches the error at the top knows neither.
            .map_err(|error| match crate::diagnostics::refusal_at(&error) {
                Some((byte, message)) => {
                    crate::diagnostics::refuse(crate::diagnostics::render_refusal(
                        &message,
                        byte,
                        &unit.path.display().to_string(),
                        &unit.source,
                    ))
                }
                None => error,
            })?;

            map.extend(body.map.placed(rust.len(), at));
            published.extend(body.published);
            rust.push_str(&body.rust);
            rust.push('\n');
        }
        // **Last, because it names lines** (ADR-300 D9). Only where the program
        // has an entry point: the table is read by the hook `fn main` installs,
        // and a package with no `main` is a library whose consumer has one.
        if self.units[0].parsed.program.items.iter().any(|item| {
            matches!(&item.node, crate::ast::Item::Fn { name: Some(name), .. }
                if self.units[0].parsed.text(*name) == "main")
        }) {
            let paths = self.shown_paths();
            let sources: Vec<&str> = self.units.iter().map(|u| u.source.as_str()).collect();
            let rows: Vec<(i64, i64, i64)> = map
                .rows()
                .map(|(at, from, unit)| (at as i64, from as i64, unit as i64))
                .collect();
            rust.push_str(&nikaia_std::tools::emit_needs::emit_needs_abort_table(
                &rust, &rows, &paths, &sources,
            ));
        }

        Ok(Lowered {
            rust,
            map,
            published,
        })
    }
}
