//! `nikaia.toml`'s build switches, and the CLI overriding them for one build.
//!
//! ADR-037 D5 and [ADR-292](../../../docs/specification/adr/adr-292.md) D7 give
//! the same shape to all three settings: the manifest carries them, because
//! they are properties of a *project* rather than of an invocation and a
//! committed value is one a reviewer sees; a flag overrides for a single build,
//! which is what a benchmark and a bug hunt need.
//!
//! Nothing here decides what a setting *means* - `emit::Build` and
//! `emit::Ordering` own that, and get handed a string either way. This module
//! only answers "which string", and does it once per run so that two places
//! cannot resolve the same setting differently.
//!
//! It also reads the rest of the manifest, for
//! [ADR-002](../../../docs/specification/adr/adr-002.md) D1's translation into
//! a `Cargo.toml`: `[package]`, `[dependencies]` and the per-target codegen
//! tables `[build.<target>]`. One reader, because a manifest read twice is a
//! manifest that can be understood two ways.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::refused;

// **Every decision the manifest makes is Nikaia** (`tools/manifest.nika`,
// ADR-294): which `[build]` keys exist, have moved or are withdrawn, which
// machines and keys `[build.<target>]` may carry, which shape a dependency
// is, and the `ManifestFile` the answers make (MF). The `toml` crate stays
// here, behind the adapter (ADR-294): this module reads the file, hands the
// document over as `ManifestItem`s of plain text, and keeps the readers'
// signatures for the callers.
use nikaia_std::tools::manifest as decide;

/// Where a `nikaia.toml` was found, and what it said: `tools/manifest.nika`'s
/// `ManifestFile`, and the `Dependency` map derived from it once.
///
/// Absent outside a project, which is not an error: a single `.nika` file
/// compiles with the built-in defaults and the flags, and littering a manifest
/// into someone's directory to make that work would be the wrong trade.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    file: decide::ManifestFile,
    dependencies: BTreeMap<String, Dependency>,
}

impl Default for Manifest {
    fn default() -> Manifest {
        Manifest {
            file: decide::manifest_empty(),
            dependencies: BTreeMap::new(),
        }
    }
}

/// One entry of `[dependencies]`, and which of the three shapes it is
/// (`tools/manifest.nika`'s `ManifestDependency`, in `toml`'s words).
///
/// The distinction is the whole of the translation: a native Rust crate is
/// passed to Cargo exactly as written (ADR-002 D1), a Nikaia package **by path**
/// is read as part of this program
/// ([ADR-286](../../../docs/specification/adr/adr-286.md) D15), and a Nikaia
/// package named any other way is something nothing has decided yet.
#[derive(Debug, Clone, PartialEq)]
pub enum Dependency {
    /// `regex = { type = "rust", version = "1.5" }` - the value with `type`
    /// removed, which is what Cargo is handed.
    Rust(toml::Value),
    /// `http = { path = "../http" }` - a Nikaia package, where it is.
    ///
    /// **The key is the name** a `use` writes, and the path says only where it
    /// comes from (ADR-286 D15 rule 1): two libraries that both want to be `http`
    /// are the consumer's to name apart, which is the authority
    /// [ADR-286](../../../docs/specification/adr/adr-286.md) D12 gives them anyway.
    /// Relative to the manifest's own directory, because that is the only
    /// interpretation that survives the project being checked out somewhere else.
    Path(PathBuf),
    /// `http-server = "1.2"`. Carried rather than resolved: a registry, a version
    /// grammar and a distribution format are the three things ADR-002 D1 §5
    /// refuses to guess at, and ADR-286 D15 keeps refusing.
    Nikaia(toml::Value),
}

/// A refusal from `tools/manifest.nika`, in the words it gave.
fn refusal(thrown: nikaia_std::error::Thrown<nikaia_std::tools::Refused>) -> anyhow::Error {
    refused!("{thrown}")
}

impl Manifest {
    /// Read the manifest governing `input`, if there is one.
    ///
    /// The search is the one the build cache already does - `Layout::resolve`
    /// walks to the nearest `nikaia.toml` at or above the input's directory -
    /// so a file cannot be cached as part of one project and compiled with
    /// another's switches. One root-finder, deliberately.
    pub fn find(input: &Path) -> Result<Manifest> {
        let layout = orchestrator::cache::Layout::resolve(input);
        if !layout.in_project {
            return Ok(Manifest::default());
        }
        Manifest::read(&layout.root.join("nikaia.toml"))
    }

    /// Read a manifest at a known path. The project build already knows where
    /// the root is, and searching again from there could only find a different
    /// answer.
    pub fn read(path: &Path) -> Result<Manifest> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut manifest = Manifest::parse(&text)
            .with_context(|| format!("in {}", path.display()))
            .map_err(|e| refused!("{e:#}"))?;
        manifest.file.root = path.parent().map(Path::to_path_buf);
        Ok(manifest)
    }

    /// Whether this manifest says anything about how to build
    /// (`manifest_has_build_section`).
    pub fn has_build_section(&self) -> bool {
        decide::manifest_has_build_section(&self.file)
    }

    /// `[package]`, `[dependencies]`, `[build]` and the `[build.<target>]`
    /// codegen tables: the document as `ManifestItem`s, read by
    /// `manifest_parse`.
    pub fn parse(text: &str) -> Result<Manifest> {
        let document: toml::Value = toml::from_str(text).context("this is not valid TOML")?;
        let file = decide::manifest_parse(&items_of(&document)).map_err(refusal)?;
        let dependencies = file
            .dependencies
            .iter()
            .map(|(name, declared)| (name.clone(), dependency_of(declared)))
            .collect();
        Ok(Manifest { file, dependencies })
    }

    /// The project root - the directory the manifest sits in.
    pub fn root(&self) -> Option<&Path> {
        self.file.root.as_deref()
    }

    /// What this manifest says that the compiler no longer reads, and where it
    /// went. One line each, for a build to print once.
    pub fn notes(&self) -> &[String] {
        &self.file.notes
    }

    /// `[package] name`, which becomes the Cargo package and the binary's name.
    pub fn package_name(&self) -> Option<&str> {
        self.file.name.as_deref()
    }

    /// `[package] version`, `0.0.0` where the project does not say.
    pub fn package_version(&self) -> &str {
        &self.file.version
    }

    /// What `tools/manifest.nika` read, for a reader in Nikaia.
    pub fn file(&self) -> &decide::ManifestFile {
        &self.file
    }

    /// Everything `[dependencies]` declared, in the order Cargo will see it.
    pub fn dependencies(&self) -> &BTreeMap<String, Dependency> {
        &self.dependencies
    }

    /// **The Rust crates this build declares**, under the name a program writes
    /// (`manifest_foreign_crates`, ADR-290 D1).
    pub fn foreign_crates(&self) -> std::collections::BTreeSet<String> {
        decide::manifest_foreign_crates(&self.file)
    }

    /// `[library.<name>]`: each library's name and its `pkg-config` name
    /// (ADR-324 D2).
    pub fn libraries(&self) -> &BTreeMap<String, String> {
        &self.file.libraries
    }

    /// The codegen table for one machine, empty where the manifest is silent.
    pub fn codegen_for(&self, target: &str) -> BTreeMap<String, toml::Value> {
        self.file
            .codegen
            .get(target)
            .map(|table| {
                table
                    .iter()
                    .map(|(key, text)| (key.clone(), value_of(text)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The effective value: the flag if one was given, else the manifest, else
    /// the built-in default.
    ///
    /// The order is the whole of D5. A flag that the manifest could override
    /// would make `--target` useless for the one build it exists for, and a
    /// manifest that the built-in default could override would make a
    /// committed switch a suggestion.
    pub fn setting<'a>(&'a self, key: &str, flag: Option<&'a str>, default: &'a str) -> &'a str {
        flag.or_else(|| self.file.build.get(key).map(String::as_str))
            .unwrap_or(default)
    }
}

/// The four sections `tools/manifest.nika` reads, their entries, and the keys
/// of an entry that is a table, in the file's order: the document in plain
/// words. Whether `--target` takes a `[build.<target>]` name is
/// `emit::Target::parse`'s answer, handed over as `machine`.
fn items_of(document: &toml::Value) -> Vec<decide::ManifestItem> {
    let mut items = Vec::new();
    for section in ["package", "dependencies", "library", "build"] {
        let Some(value) = document.get(section) else {
            continue;
        };
        items.push(item(section, "", "", value, false));
        for (name, entry) in value.as_table().into_iter().flatten() {
            let machine =
                section == "build" && entry.is_table() && crate::emit::Target::parse(name).is_ok();
            items.push(item(section, name, "", entry, machine));
            for (key, inner) in entry.as_table().into_iter().flatten() {
                items.push(item(section, name, key, inner, false));
            }
        }
    }
    items
}

fn item(
    section: &str,
    name: &str,
    key: &str,
    value: &toml::Value,
    machine: bool,
) -> decide::ManifestItem {
    decide::ManifestItem {
        section: section.to_string(),
        name: name.to_string(),
        key: key.to_string(),
        table: value.is_table(),
        text: value.to_string(),
        word: value.as_str().map(str::to_string),
        integer: value.as_integer(),
        machine,
    }
}

/// A value from the text `toml` wrote of it (`ManifestItem::text`).
fn value_of(text: &str) -> toml::Value {
    let mut table: toml::Table =
        toml::from_str(&format!("v = {text}")).expect("a value's text reads back");
    table.remove("v").expect("the value is there")
}

/// The `Dependency` of a `ManifestDependency`: a Rust crate's table without
/// its `type`, Nikaia's word about which ecosystem the name belongs to, which
/// Cargo would reject as an unknown key.
fn dependency_of(declared: &decide::ManifestDependency) -> Dependency {
    let value = value_of(&declared.text);
    match (declared.shape, &declared.path) {
        (decide::DependencyShape::Rust, _) => {
            let mut table = value
                .as_table()
                .cloned()
                .expect("a value with a `type` key is a table");
            table.remove("type");
            Dependency::Rust(toml::Value::Table(table))
        }
        (decide::DependencyShape::Path, Some(path)) => Dependency::Path(PathBuf::from(path)),
        _ => Dependency::Nikaia(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The other tests of this module moved to `tools/manifest.nika` (ADR-294,
    // #125). These read `toml::Value`s, a flag, or a tuple with a nullable slot
    // that a list literal cannot yet lower.

    #[test]
    fn a_flag_beats_the_manifest_and_the_manifest_beats_the_default() {
        let manifest = Manifest::parse("[build]\ntarget = \"wasm32-unknown\"\n").expect("parses");
        assert_eq!(
            manifest.setting("target", None, "x86_64-linux"),
            "wasm32-unknown"
        );
        assert_eq!(
            manifest.setting("target", Some("x86_64-linux"), "x86_64-linux"),
            "x86_64-linux"
        );
        assert_eq!(
            manifest.setting("ordering", None, "effects"),
            "effects",
            "a key the manifest does not carry falls through to the default"
        );
    }

    /// ADR-002 D6: `codegen-units` is a whole number from 1, and anything else
    /// is refused saying so rather than handed to Cargo.
    #[test]
    fn codegen_units_is_a_whole_number_from_one() {
        let manifest = Manifest::parse("[build.x86_64-linux]\ncodegen-units = 1\n")
            .expect("one unit is a count");
        assert_eq!(
            manifest.codegen_for("x86_64-linux")["codegen-units"].as_integer(),
            Some(1)
        );
        for written in ["0", "\"16\"", "1.5", "-3"] {
            let error = Manifest::parse(&format!(
                "[build.x86_64-linux]\ncodegen-units = {written}\n"
            ))
            .expect_err("not a count from 1");
            let text = format!("{error:#}");
            assert!(text.contains("whole number from 1"), "{written}: {text}");
            assert!(text.contains(written), "{written}: {text}");
        }
    }

    /// Part III 13.3's two shapes. `type = "rust"` says crates.io, and the
    /// marker is Nikaia's - Cargo would refuse it as an unknown key, so it does
    /// not travel (ADR-002 D1).
    #[test]
    fn a_rust_dependency_passes_through_without_its_marker() {
        let manifest = Manifest::parse(
            "[dependencies]\nhttp-server = \"1.2\"\nregex = { type = \"rust\", version = \"1.5\" }\n",
        )
        .expect("parses");

        match &manifest.dependencies()["regex"] {
            Dependency::Rust(value) => {
                assert_eq!(
                    value.get("version").and_then(toml::Value::as_str),
                    Some("1.5")
                );
                assert!(
                    value.get("type").is_none(),
                    "the marker does not reach Cargo"
                );
            }
            other => panic!("regex is a Rust crate, not {other:?}"),
        }
        assert!(matches!(
            manifest.dependencies()["http-server"],
            Dependency::Nikaia(_)
        ));
    }
}
