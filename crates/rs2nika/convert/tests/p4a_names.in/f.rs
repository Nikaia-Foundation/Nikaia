/// Stage 0: parse, check, lower, infer - or serve all four from the cache.
///
/// The build cache (ADR-021) covers the whole of it: on a hit nothing is
/// checked, lowered or inferred, because only a build that passed every check
/// was recorded and the key holds everything those checks depend on (D13).
/// **The packages a project depends on by path**
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D15), resolved against
/// the manifest's own directory.
///
/// A free function because two callers need the same answer: the project build,
/// and the `rustc` wrapper, which is handed a `.nika` path by Cargo and has to
/// find the manifest above it (`wrapper_main`). Two copies of a resolution rule
/// is two chances to disagree about what a program is.
///
/// Three of D2's five rules are kept here, because they are about the set of
/// dependencies rather than about any one of them:
///
/// * **rule 1**, the manifest key is the name: the map's key is what `use`
///   writes and the path is only where it comes from, so nothing inside a
///   package gets to name it;
/// * **rule 3**, the same path is the same package: two keys resolving to one
///   directory would be two `mod`s of one package and two types of one name,
///   so it is refused rather than unified;
/// * **rule 4**, a dependency's `[build]` is ignored, and the compiler says
///   so — a package is built with the settings of the program that uses it,
///   and anything else puts two answers to the parallelism question in one
///   build.
///
/// **rule 2** — transitive dependencies are not visible — is no longer checked
/// anywhere, because it is structural
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D21): a package is
/// generated as its own crate naming only its own dependencies, so a name from
/// two levels down does not resolve and no rule of ours has to say so.
/// **rule 5** is the generated workspace's profile (D4): every Nikaia crate is
/// named on the program's side of the overflow checks, beside the exception
/// that turns them off for every foreign package.
pub fn packages_of(
    manifest: &Manifest,
    root: &Path,
    announce: bool,
) -> Result<Vec<modules::Dependency>> {
    let mut out: Vec<modules::Dependency> = Vec::new();
    let mut seen: BTreeMap<PathBuf, String> = BTreeMap::new();

    // **What this build calls each package**, before any of them is read, so
    // that a package reached twice is renamed to one name rather than to
    // whichever of the two was resolved first
    // ([ADR-286](../../docs/specification/adr/adr-286.md) D20).
    let mut here: BTreeMap<PathBuf, String> = BTreeMap::new();
    for (name, value) in manifest.dependencies() {
        if let Dependency::Path(path) = value {
            let at = root.join(path);
            here.insert(at.canonicalize().unwrap_or(at), name.clone());
        }
    }

    for (name, value) in manifest.dependencies() {
        let Dependency::Path(path) = value else {
            continue;
        };
        let root = root.join(path);
        let root = root.canonicalize().unwrap_or(root);

        if let Some(first) = seen.get(&root) {
            refuse!(
                "`{name}` and `{first}` are the same package: both are {}.\n\
                 The same path is the same package, so two names for it \
                 would be two copies of every type it declares - name it once.",
                root.display()
            );
        }
        seen.insert(root.clone(), name.clone());

        // **A package's own packages are resolved, not refused**
        // (ADR-286 D19): it is generated as its own crate, so they are its
        // dependencies and not ours. Read here only to fail early where one is
        // not there.
        let dependency = Manifest::read(&root.join("nikaia.toml"))?;
        // `announce`, because this resolution runs more than once per build - the
        // driver, and the `rustc` wrapper Cargo calls for the build and again for
        // a run - and a note a reader meets three times reads as three problems.
        // The driver is the one that reports (ADR-005 D7's shape: the work happens
        // where it happens, the saying happens once).
        if announce && dependency.has_build_section() {
            eprintln!(
                "note: `{}`'s own `[build]` is ignored: a package is built with the \
                 settings of the program that uses it.",
                name
            );
        }

        out.push(modules::Dependency {
            name: name.clone(),
            renames: renames_in(&dependency, &root, &here)?,
            root,
            // **Its own keys**, so its `use` lines are read the way its own
            // build would read them (ADR-286 D21).
            // A Rust crate among them under the name a call writes it by.
            reachable: dependency
                .dependencies()
                .keys()
                .cloned()
                .chain(dependency.foreign_crates())
                .collect(),
        });
    }
    Ok(out)
}

/// How a dependency's **own** package names are read in this build
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D20: a package reached
/// through two parents is one package).
///
/// A library writes its own manifest keys in its own signatures, and those keys
/// are its author's choice rather than this program's. So a program that depends
/// on `deep` directly, and on a library that depends on the same directory as
/// `c`, was told its `deep::Id` was not the `c::Id` that library takes - one
/// type, refused for having two spellings, which is the one thing D2 says it is
/// not. The types are identical in the generated Rust; only the name this
/// compiler gave them differed.
///
/// So the identity is the **package**, and this is the translation into the word
/// this build uses for it: the program's own key where the program names it too,
/// and otherwise the package's `[package] name`, which `members_of` has already
/// refused to let two packages share. Either way it is a function of the
/// directory and not of who asked.
///
/// One level deep, because one level is what is read: a dependency's own
/// dependencies are its crate's business (D3), and nothing below it is in this
/// program's ledger to be renamed.
fn renames_in(
    dependency: &Manifest,
    dependency_root: &Path,
    here: &BTreeMap<PathBuf, String>,
) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for (key, value) in dependency.dependencies() {
        let Dependency::Path(path) = value else {
            continue;
        };
        let at = dependency_root.join(path);
        let at = at.canonicalize().unwrap_or(at);
        let canonical = match here.get(&at) {
            Some(ours) => ours.clone(),
            None => Manifest::read(&at.join("nikaia.toml"))?
                .package_name()
                .map(str::to_string)
                .unwrap_or_else(|| key.clone()),
        };
        if &canonical != key {
            out.insert(key.clone(), canonical);
        }
    }
    Ok(out)
}

/// One package of a build, and everything needed to generate a crate for it
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D19).
#[derive(Debug, Clone)]
pub struct Member {
    /// The Cargo package name - the `[package] name` of its own `nikaia.toml`.
    /// Its **identity** is `root`; this is what the profile rows and the
    /// renaming form below name it by.
    pub name: String,
    /// The canonical package directory. Two manifest keys resolving here are
    /// one crate, which is [ADR-286](../../docs/specification/adr/adr-286.md)
    /// D2 rule 3 holding because Cargo already works that way (D2).
    pub root: PathBuf,
    pub manifest: Manifest,
    /// The `.nika` file Cargo is told is the crate root. `src/main.nika` where
    /// there is one; a library needs none, so the first source beside it stands
    /// in - the wrapper walks up from whatever it is handed, so any file of the
    /// package finds the same manifest.
    pub entry: PathBuf,
    /// Its **own** packages, under its **own** manifest keys.
    pub dependencies: Vec<modules::Dependency>,
}

/// The members in an order where every package comes after the packages it
/// depends on ([ADR-100](../../docs/specification/adr/adr-100.md) D5).
///
/// **Why not simply reverse `members_of`.** That walk is breadth-first from the
/// entry, and reversing it is a topological order only where nothing is reached
/// two ways: with `a → b`, `a → c` and `c → b`, breadth-first gives `a, b, c`
/// and the reverse puts `c` before `b`, which is exactly backwards. So this is
/// Kahn's, over the edges the members already carry.
///
/// **A cycle keeps the order it had.** Two packages that depend on each other
/// are refused below this — Cargo will not build the workspace — and a
/// compiler that hung or panicked on the way to that message would be reporting
/// the wrong thing. What comes out is every member exactly once either way,
/// which is what the caller needs of it.
fn dependencies_first(members: &[Member]) -> Vec<usize> {
    let at_of: BTreeMap<&Path, usize> = members
        .iter()
        .enumerate()
        .map(|(at, m)| (m.root.as_path(), at))
        .collect();

    // `needs[at]` is what `at` waits for; `feeds[at]` is who waits for it.
    let needs: Vec<BTreeSet<usize>> = members
        .iter()
        .map(|m| {
            m.dependencies
                .iter()
                .filter_map(|d| at_of.get(canonical(&d.root).as_path()).copied())
                .collect()
        })
        .collect();

    let mut left: Vec<usize> = needs.iter().map(|n| n.len()).collect();
    // Ready in `members_of`'s order, so the result is a function of the source
    // tree and not of a set's iteration (Part III 13.5).
    let mut ready: Vec<usize> = (0..members.len()).filter(|at| left[*at] == 0).collect();
    let mut out: Vec<usize> = Vec::with_capacity(members.len());

    while let Some(at) = ready.first().copied() {
        ready.remove(0);
        out.push(at);
        for (other, waits_for) in needs.iter().enumerate() {
            if waits_for.contains(&at) {
                left[other] -= 1;
                if left[other] == 0 {
                    ready.push(other);
                }
            }
        }
    }

    // A cycle leaves members unplaced. They go on the end in the order they
    // came, so every member is lowered exactly once and the refusal that is
    // really about this arrives from where it belongs.
    let unplaced: Vec<usize> = (0..members.len()).filter(|at| !out.contains(at)).collect();
    out.extend(unplaced);
    out
}

/// Every package of this build, the entry first
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D19).
///
/// Breadth-first from the entry, so a diamond is visited once and the order is
/// the same on every machine. A package already seen is the same crate: the key
/// under which it was reached is the *depending* crate's business (D2), and
/// nothing here is named by it.
pub fn members_of(entry_manifest: &Manifest, entry_root: &Path) -> Result<Vec<Member>> {
    let mut out: Vec<Member> = Vec::new();
    let mut seen: BTreeMap<PathBuf, usize> = BTreeMap::new();
    let mut queue: Vec<(Manifest, PathBuf)> = vec![(entry_manifest.clone(), canonical(entry_root))];

    while let Some((manifest, root)) = queue.first().cloned() {
        queue.remove(0);
        if seen.contains_key(&root) {
            continue;
        }

        let name = manifest
            .package_name()
            .ok_or_else(|| {
                refused!(
                    "{}/nikaia.toml has no `[package] name`. Add one: every package needs \
                     a name.",
                    root.display()
                )
            })?
            .to_string();

        // A crate name is a package's identity in the generated workspace, and
        // two of them would be two answers to which crate the profile rows and
        // the renaming form below mean. The canonical path is the identity, so
        // this is only ever two *different* packages wanting one name.
        if let Some(first) = out.iter().find(|m| m.name == name) {
            refuse!(
                "Two packages are both called `{name}`: {} and {}.\n\
                 Give one of them another `[package] name`. The name a `use` line writes \
                 is set in `[dependencies]` and doesn't have to change.",
                first.root.display(),
                root.display()
            );
        }

        // Announced only for the entry: this walk reads every manifest of the
        // build, and a note about an ignored `[build]` is about the package
        // that carries it, said once.
        let dependencies = packages_of(&manifest, &root, out.is_empty())?;
        for dependency in &dependencies {
            let manifest = Manifest::read(&dependency.root.join("nikaia.toml"))?;
            queue.push((manifest, canonical(&dependency.root)));
        }

        seen.insert(root.clone(), out.len());
        out.push(Member {
            name,
            entry: crate_root_of(&root)?,
            root,
            manifest,
            dependencies,
        });
    }
    Ok(out)
}
fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// The `.nika` file handed to Cargo as a package's crate root.
///
/// `src/main.nika` where there is one. A library has none - what a package
/// offers is its public surface (`modules::package_at`) - and Cargo still needs
/// a path that exists, so the first source beside it stands in. Which file it
/// is changes nothing: the wrapper walks up from it to the same `nikaia.toml`,
/// and `package_at` reads the whole directory either way.
fn crate_root_of(root: &Path) -> Result<PathBuf> {
    let entry = root.join(ENTRY);
    if entry.is_file() {
        return Ok(entry);
    }

    let src = root.join("src");
    let mut beside: Vec<PathBuf> = std::fs::read_dir(&src)
        .with_context(|| format!("cannot read {}", src.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == EXTENSION))
        .collect();
    beside.sort();

    beside.into_iter().next().ok_or_else(|| {
        refused!(
            "{} has no `.nika` files, so there's nothing to build.\n\
             Put the package's source files in its `src/` folder.",
            src.display()
        )
    })
}

