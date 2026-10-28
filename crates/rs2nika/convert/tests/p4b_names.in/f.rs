/// Everything the compiler decides for itself, before it emits a line of Rust.
///
/// Two rules today, and one mechanism under both: the ledger (ADR-020) is what
/// makes either possible across the `std` boundary. A call to
/// `io::read_to_string` is a `sync` violation only if something says that
/// function can pause, and it takes the wrong number of arguments only if
/// something says how many it takes - `std.contracts` is where both are said.
///
/// Types are reported before suspension because a call that passes the wrong
/// thing is usually why the rest of the file reads strangely.
///
/// **What this build links against, and what has been described**
/// ([ADR-290](../../docs/specification/adr/adr-290.md) D1).
///
/// Two sets rather than one map, because the question is a difference: a crate
/// the manifest declares and no ledger describes is the one D1 refuses. Both
/// empty is a loose file with no project around it, where nothing was declared
/// and therefore nothing is undescribed.
///
/// **And the entries themselves**, which is D1's first sentence rather than an
/// addition to it: *every analysis reaches to the boundary and reads an entry
/// there*. `NK2504`'s own message promises a reader that four questions — what
/// may cross a thread, what the call may reach, whether it pauses, whether it
/// can fail — are answered by the file they are told to write. For two records
/// the file answered none of them: it was parsed to see whether it parsed, the
/// answer was thrown away, and its only effect was silencing the refusal that
/// asked for it. Measured on a three-line project, where a description saying
/// `() -> i64` left the call's result untyped and `n.no_such_method()` went
/// through.
#[derive(Debug, Clone)]
pub struct Foreign {
    /// `[dependencies]` with `type = "rust"`, under the name a program writes
    /// (`manifest::foreign_crates`).
    pub declared: BTreeSet<String>,
    /// The crates a `contracts/<crate>.contracts` was found for, beside the
    /// project's own ledger (D5).
    pub described: BTreeSet<String>,
    /// The described crates whose **sources have moved** since the description
    /// was reviewed, with the files that hash differently
    /// ([ADR-290](../../docs/specification/adr/adr-290.md) D5, on
    /// [ADR-100](../../docs/specification/adr/adr-100.md) D3's rule).
    ///
    /// **Empty where nothing can be compared**, which is the polarity and not
    /// an omission: a version dependency's sources are in Cargo's registry
    /// cache and a description with no `[sources]` recorded nothing, so in both
    /// cases there is no hash to disagree with. A refusal resting on an absence
    /// is what [ADR-281](../../docs/specification/adr/adr-281.md) D34 keeps
    /// `NK2201` from doing, and it would here refuse every crate that comes
    /// from a registry — [Part III C.4](../../docs/specification/30-nikaia-tooling.md)'s
    /// correct program refused.
    pub moved: BTreeMap<String, Vec<String>>,
    /// Every described crate's entries, in one ledger under the qualified names
    /// a program writes (`hyper_shim::serve_once`).
    ///
    /// **One ledger and not one per crate**, because a name is resolved in one
    /// place (ADR-296 D17) and the crate a name belongs to is the prefix on it.
    /// Two descriptions cannot disagree about a name without disagreeing about
    /// whose it is.
    pub descriptions: Ledger,
    /// **The Rust crates the Nikaia packages beneath this one declare**
    /// ([ADR-286](../../docs/specification/adr/adr-286.md) D31, #488). A
    /// package's files are checked in this build, and `shim::plus_one` in a
    /// dependency names a head that dependency's manifest declares; this build's
    /// own manifest does not. Only a head: what the crate's items are is the
    /// dependency's description, and nothing here reads it.
    pub beneath: BTreeSet<String>,
}

impl Default for Foreign {
    fn default() -> Self {
        Foreign {
            declared: BTreeSet::new(),
            described: BTreeSet::new(),
            moved: BTreeMap::new(),
            descriptions: Ledger::blank(),
            beneath: BTreeSet::new(),
        }
    }
}

/// The Rust crates every path dependency of `root` declares, and theirs in
/// turn. A manifest that cannot be read adds nothing: the build reads it
/// itself and says so.
fn crates_beneath(root: &Path, seen: &mut BTreeSet<PathBuf>, out: &mut BTreeSet<String>) {
    let Ok(manifest) = Manifest::read(&root.join("nikaia.toml")) else {
        return;
    };
    for value in manifest.dependencies().values() {
        let Dependency::Path(path) = value else {
            continue;
        };
        let at = root.join(path);
        let at = at.canonicalize().unwrap_or(at);
        if !seen.insert(at.clone()) {
            continue;
        }
        if let Ok(theirs) = Manifest::read(&at.join("nikaia.toml")) {
            out.extend(theirs.foreign_crates());
        }
        crates_beneath(&at, seen, out);
    }
}

impl Foreign {
    /// What a build of `root` is at its boundary.
    ///
    /// A manifest that cannot be read is **no declarations**, not a failure: the
    /// build has read it already and said so, and a second refusal from here
    /// would be the same mistake twice.
    pub fn of(root: &Path) -> Foreign {
        let declared = crate::manifest::Manifest::read(&root.join("nikaia.toml"))
            .map(|manifest| manifest.foreign_crates())
            .unwrap_or_default();
        let mut described = BTreeSet::new();
        let mut descriptions = Ledger::empty();
        let mut moved = BTreeMap::new();
        for name in &declared {
            let Some(ledger) = description_at(root, name) else {
                continue;
            };
            described.insert(name.clone());
            let differing = sources_that_moved(root, name, &ledger);
            if !differing.is_empty() {
                moved.insert(name.clone(), differing);
            }
            // `None`: a description's names are already qualified with the crate
            // word a program writes (`hyper_shim::serve_once`), so there is
            // nothing to put in front of them.
            descriptions.absorb(None, ledger);
        }
        let mut beneath = BTreeSet::new();
        crates_beneath(root, &mut BTreeSet::new(), &mut beneath);
        Foreign {
            declared,
            described,
            descriptions,
            moved,
            beneath,
        }
    }
    /// The words a program may write in front of a `::` **because the manifest
    /// declared them**, added to the ones its own packages give it.
    ///
    /// Without this the merge above would make `hyper_shim` a **`std` module**,
    /// since that set is derived from the library ledger's own key prefixes —
    /// and the first thing a program calling one would be told is to write
    /// `use std::hyper_shim`, which is not a sentence about anything. A crate is
    /// reached because the manifest declares it (D1), not because a file
    /// imported it.
    pub fn packages(&self, modules: &BTreeSet<String>) -> BTreeSet<String> {
        let mut out = modules.clone();
        out.extend(self.described.iter().cloned());
        // **A crate the manifest declares is a head** (ADR-286 D31), described
        // or not: an undescribed one is `NK2504`'s to refuse, with its way out.
        out.extend(self.declared.iter().cloned());
        out.extend(self.beneath.iter().cloned());
        out
    }
}
/// Which of the files a description was derived from hash differently now
/// ([ADR-290](../../docs/specification/adr/adr-290.md) D5).
///
/// **Only what can be compared is compared.** A description that recorded no
/// `[sources]` and a crate whose sources this build cannot find both answer
/// *nothing moved*, because there is no hash to disagree with — the honest
/// answer, and the one that keeps a registry crate from being refused for
/// coming from a registry (Part III, C.4).
fn sources_that_moved(root: &Path, name: &str, ledger: &Ledger) -> Vec<String> {
    if ledger.sources.is_empty() {
        return Vec::new();
    }
    let Some(crate_root) = crate::describe::crate_root(root, name) else {
        return Vec::new();
    };
    ledger
        .sources
        .iter()
        .filter(|(file, recorded)| {
            // **A file that cannot be read is not a file that moved.** A
            // checkout without the path dependency beside it is a build that
            // will fail for its own reasons, and a second message about it
            // would be this compiler guessing at the first.
            std::fs::read(crate_root.join(file))
                .is_ok_and(|bytes| orchestrator::cache::sha256_hex(&bytes) != **recorded)
        })
        .map(|(file, _)| file.clone())
        .collect()
}

/// The digest of `nikaia.proofs` and of every certificate in `nikaia.proofs.d/`,
/// empty where there is no file (#448).
fn proofs_digest(root: &Path) -> String {
    let Ok(book) = std::fs::read(root.join("nikaia.proofs")) else {
        return String::new();
    };
    let mut all = book;
    if let Ok(entries) = std::fs::read_dir(root.join("nikaia.proofs.d")) {
        let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        files.sort();
        for file in files {
            all.extend(
                file.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
                    .bytes(),
            );
            all.extend(std::fs::read(&file).unwrap_or_default());
        }
    }
    orchestrator::cache::sha256_hex(&all)
}

/// `contracts/<crate>.contracts`, where it is there **and parses as a ledger**.
///
/// Parsing is the test rather than existence, for
/// [ADR-100](../../docs/specification/adr/adr-100.md) D3's reason one file over:
/// a file that does not parse is not an answer, and treating it as one would
/// let a boundary be described by something nobody can read.
fn description_at(root: &Path, name: &str) -> Option<Ledger> {
    Ledger::read_beside(&root.join("contracts").join(format!("{name}.contracts")))
}
