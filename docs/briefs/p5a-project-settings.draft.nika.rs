impl Settings {
    /// Whether this build makes a library C calls (ADR-284 D4).
    pub fn library(&self) -> bool {
        self.words.artifact == "c-library"
    }

    /// Resolve every switch before anything is read, so a mistyped one fails on
    /// its own account rather than after a compile (ADR-037 D5).
    pub fn resolve(
        manifest: &Manifest,
        target: Option<&str>,
        user_parallelism: Option<&str>,
    ) -> Result<Settings> {
        let target = manifest
            .setting("target", target, crate::emit::default_target().name())
            .to_string();
        let user_parallelism = manifest
            .setting("user-parallelism", user_parallelism, "no")
            .to_string();
        // **No flag of its own** ([ADR-281](../../docs/specification/adr/adr-281.md)
        // D8): it *lives in the manifest*, because otherwise a shipped build is
        // not reproducible, and Part I 1.2 names the two that a single build may
        // override. The environment is not a second front door — it is how the
        // resolved word reaches the wrapper, which Cargo owns the arguments of.
        let reentrancy_check = manifest
            .setting("reentrancy-check", None, "yes")
            .to_string();
        let optimization = manifest.setting("optimization", None, "").to_string();
        let refuted_claims = refuted_claims(manifest.setting("refuted-claims", None, "error"))?;
        let unproved_claims = unproved_claims(manifest.setting("unproved-claims", None, "error"))?;
        let artifact = artifact(manifest.setting("artifact", None, "program"))?;
        let symbol_prefix = symbol_prefix(
            manifest.setting(
                "symbol-prefix",
                None,
                &manifest
                    .package_name()
                    .unwrap_or("program")
                    .replace('-', "_"),
            ),
        )?;
        let mut build = Build::parse(&target, &user_parallelism, &reentrancy_check)?;
        (build.bounds, build.overflow) = optimizations(&optimization)?;
        build.skip_free_at_exit = skip_free_at_exit(&optimization);
        build.prove_through_loops = prove_through_loops(&optimization);
        build.called_from_c = artifact == "c-library";
        Ok(Settings {
            build,
            words: ProjectWords {
                target,
                user_parallelism,
                reentrancy_check,
                optimization,
                refuted_claims,
                unproved_claims,
                artifact,
                symbol_prefix,
                tests: false,
                bind: None,
                cpu: None,
            },
        })
    }

    /// **The command line's `--optimization` over the manifest's**
    /// ([ADR-306](../../docs/specification/adr/adr-306.md) D1). Nothing a
    /// program means depends on it, so a single build may say it, as it may
    /// say `--target`.
    /// **The command line's `--refuted-claims` over the manifest's**
    /// ([ADR-269](../../docs/specification/adr/adr-269.md) D8): a single build
    /// may let through a claim a newer prover refutes, as it may name another
    /// target.
    /// **The command line's `--unproved-claims` over the manifest's**
    /// ([ADR-380](../../docs/specification/adr/adr-380.md) D2), the same way.
    pub fn refutations(&mut self, refuted: Option<&str>, unproved: Option<&str>) -> Result<()> {
        if let Some(word) = refuted {
            self.words.refuted_claims = refuted_claims(word)?;
        }
        if let Some(word) = unproved {
            self.words.unproved_claims = unproved_claims(word)?;
        }
        Ok(())
    }

    /// **`--cpu=LEVEL`** ([ADR-002](../../docs/specification/adr/adr-002.md)
    /// D6): one of the target's levels, or the build fails naming them. The
    /// baseline is the same build as no level at all.
    pub fn choose_cpu(&mut self, level: Option<&str>) -> Result<()> {
        let Some(level) = level else {
            return Ok(());
        };
        let target = self.build.target;
        let levels = crate::sysroot::cpu_levels(target.triple())?;
        if !levels.iter().any(|known| known == level) {
            let named = levels
                .iter()
                .map(|known| format!("`{known}`"))
                .collect::<Vec<_>>()
                .join(", ");
            refuse!(
                "`--cpu={level}` is not a CPU level of `{}`. Its levels are: {named}.",
                target.name()
            );
        }
        self.words.cpu = (level != levels[0]).then(|| level.to_string());
        Ok(())
    }

    /// The CPU level as a dimension of the compiled `std` (ADR-002 D6): empty
    /// for the baseline, the level's name, and for `native` the CPU `rustc`
    /// finds on this machine, never the word.
    pub fn cpu_key(&self) -> Result<String> {
        match self.words.cpu.as_deref() {
            None => Ok(String::new()),
            Some("native") => crate::sysroot::native_cpu(self.build.target.triple()),
            Some(level) => Ok(level.to_string()),
        }
    }

    /// What Cargo is told for the CPU level (ADR-002 D6, ADR-325).
    pub fn cpu_config(&self) -> Vec<String> {
        project_settings::cpu_config(&self.words, self.build.target)
    }

    /// **`--cpu` against the user's `RUSTFLAGS`** (#753, ADR-002 D6, Part III
    /// 13.3). Cargo ignores the `rustflags` of `cpu_config` when `RUSTFLAGS` or
    /// `CARGO_ENCODED_RUSTFLAGS` is set, so then the child gets
    /// `CARGO_ENCODED_RUSTFLAGS`: the user's flags, then `-C target-cpu=L` last
    /// (the level wins, the flags still apply). `None` where nothing is needed.
    pub fn cpu_rustflags(&self) -> Result<Option<String>> {
        if self.words.cpu.is_none() {
            return Ok(None);
        }
        let encoded = std::env::var("CARGO_ENCODED_RUSTFLAGS").ok();
        let plain = std::env::var("RUSTFLAGS").ok();
        Ok(project_settings::encoded_with_cpu(
            &encoded,
            &plain,
            &self.cpu_key()?,
        ))
    }

    pub fn optimize(&mut self, flags: &[String]) -> Result<()> {
        if flags.is_empty() {
            return Ok(());
        }
        let words = flags.join(",");
        (self.build.bounds, self.build.overflow) = optimizations(&words)?;
        self.build.skip_free_at_exit = skip_free_at_exit(&words);
        self.build.prove_through_loops = prove_through_loops(&words);
        self.words.optimization = words;
        Ok(())
    }

    /// The words, for the wrapper. Cargo owns the wrapper's arguments, so the
    /// environment is the only channel the switches can travel down - and they
    /// travel already resolved, because a second resolution is a second chance
    /// to disagree.
    fn as_env(&self) -> Vec<(String, OsString)> {
        vec![
            (TARGET_VAR.to_string(), OsString::from(&self.words.target)),
            (
                PARALLELISM_VAR.to_string(),
                OsString::from(&self.words.user_parallelism),
            ),
            (
                REENTRANCY_VAR.to_string(),
                OsString::from(&self.words.reentrancy_check),
            ),
            (
                OPTIMIZATION_VAR.to_string(),
                OsString::from(&self.words.optimization),
            ),
            (
                REFUTED_VAR.to_string(),
                OsString::from(&self.words.refuted_claims),
            ),
            (
                UNPROVED_VAR.to_string(),
                OsString::from(&self.words.unproved_claims),
            ),
            (
                ARTIFACT_VAR.to_string(),
                OsString::from(&self.words.artifact),
            ),
            (
                PREFIX_VAR.to_string(),
                OsString::from(&self.words.symbol_prefix),
            ),
        ]
        .into_iter()
        .chain(
            self.words
                .tests
                .then(|| (TESTS_VAR.to_string(), OsString::from("1"))),
        )
        .chain(
            self.words
                .cpu
                .as_ref()
                .map(|level| (CPU_VAR.to_string(), OsString::from(level))),
        )
        .collect()
    }

    /// What the driver put in the environment. Absent means the wrapper was run
    /// by hand, and the built-in defaults are the honest answer.
    fn from_env() -> Result<Settings> {
        let word =
            |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.to_string());
        let target = word(TARGET_VAR, crate::emit::default_target().name());
        let user_parallelism = word(PARALLELISM_VAR, "no");
        let reentrancy_check = word(REENTRANCY_VAR, "yes");
        let optimization = word(OPTIMIZATION_VAR, "");
        let refuted_claims = refuted_claims(&word(REFUTED_VAR, "error"))?;
        let unproved_claims = unproved_claims(&word(UNPROVED_VAR, "error"))?;
        let artifact = artifact(&word(ARTIFACT_VAR, "program"))?;
        let symbol_prefix = symbol_prefix(&word(PREFIX_VAR, "program"))?;
        let mut build = Build::parse(&target, &user_parallelism, &reentrancy_check)?;
        (build.bounds, build.overflow) = optimizations(&optimization)?;
        build.skip_free_at_exit = skip_free_at_exit(&optimization);
        build.prove_through_loops = prove_through_loops(&optimization);
        build.called_from_c = artifact == "c-library";
        Ok(Settings {
            build,
            words: ProjectWords {
                target,
                user_parallelism,
                reentrancy_check,
                optimization,
                refuted_claims,
                unproved_claims,
                artifact,
                symbol_prefix,
                tests: std::env::var_os(TESTS_VAR).is_some(),
                bind: None,
                cpu: std::env::var(CPU_VAR)
                    .ok()
                    .filter(|level| !level.is_empty()),
            },
        })
    }

    /// The cache's view of this build (ADR-021 D5): the key is
    /// `project_settings::choices_key`'s, and the backend is `"rust"` because it
    /// names **the lowering that produced the artifact**, not the flag the user
    /// typed, and a bare invocation *is* `--backend rust`
    /// ([ADR-004](../../docs/specification/adr/adr-004.md) D1), so the two
    /// spellings share one entry - which is correct, because it is one lowering.
    /// The dimension itself stays in [`orchestrator::cache::Key::build`] (D5, D7)
    /// so that the day a second backend caches, the literal here becomes that
    /// backend's name and the two cannot collide.
    pub fn choices(&self) -> Choices {
        let cpu_key = match self.words.cpu {
            Some(_) => self.cpu_key().unwrap_or_default(),
            None => String::new(),
        };
        let build = &self.build;
        Choices::new(
            project_settings::choices_key(
                &self.words,
                build.bounds,
                build.overflow,
                build.skip_free_at_exit,
                build.prove_through_loops,
                &cpu_key,
            ),
            "rust",
        )
    }

    /// What a panic does on this machine (ADR-037 D1), in Cargo's vocabulary.
    fn panic_strategy(&self) -> String {
        project_settings::panic_strategy(self.build.target)
    }
}
/// A project: its root, its manifest, and the switches this build resolved.
#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub settings: Settings,
    /// **The warnings were said already** (#481): `nikaia run` with a file
    /// lowers it where it stands first, so its findings name the file the
    /// author wrote, and the build of the copy then keeps them to itself.
    pub warned: bool,
    /// **The file a loose `nikaia run` was given** (#510): the build is of a
    /// copy in the cache, and a message the language below raised about it is
    /// said against the file the author wrote, not the copy's path.
    pub written: Option<PathBuf>,
    /// **Which of `nikaia test`'s builds this is** (ADR-269 D12): the test
    /// build and the program, each at each setting asked for. They differ in
    /// what is lowered and in the switches, so in one place each would make
    /// the others stale and Cargo would build all of them again every run;
    /// each keeps a directory and a name of its own instead. `None` for a
    /// plain build, which keeps `target/nikaia/` as it is.
    pub variant: Option<String>,
}

thread_local! {
    /// Set while a build lowers what [`Project::warned`] said already.
    static WARNED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Set while a build lowers the copy [`Project::written`] names: the entry
    /// it lowers is said under that file's name (#510).
    static SHOWN_AS: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

impl Project {
    /// Find the project governing `start` and read its manifest.
    ///
    /// The walk to the root is `Layout::resolve`'s - the same one the build
    /// cache uses - so a build and the cache entries it writes can never
    /// disagree about which project a file belongs to.
    pub fn open(
        start: &Path,
        target: Option<&str>,
        user_parallelism: Option<&str>,
    ) -> Result<Project> {
        // `Layout::resolve` searches from an input file's *parent*, so it is
        // handed the manifest's own path: the search then starts at `start`
        // itself, which is what "build the project I am standing in" means.
        let layout = Layout::resolve(&start.join("nikaia.toml"));
        if !layout.in_project {
            refuse!(
                "There's no `nikaia.toml` in {} or any folder above it.\n\
                 `nikaia build` needs a project. To lower a single file to Rust, \
                 run `nikaia lower {}`.",
                start.display(),
                start.join("main.nika").display()
            );
        }

        let manifest = Manifest::read(&layout.root.join("nikaia.toml"))?;
        // ADR-303 D5 moved `cleanup-deadline` out of the manifest. The key is
        // still accepted, and the note is what keeps the move from being
        // silent - a manifest whose setting stopped being read without saying
        // so is the mistake `[build]`'s unknown-key check already exists for.
        for note in manifest.notes() {
            eprintln!("note: {note}");
        }
        let settings = Settings::resolve(&manifest, target, user_parallelism)?;
        if let Some(missing) = settings.build.target.unbuildable() {
            refuse!(
                "Building for `{}` isn't possible yet: {missing}",
                settings.build.target.triple()
            );
        }

        Ok(Project {
            root: layout.root,
            manifest,
            settings,
            warned: false,
            written: None,
            variant: None,
        })
    }

    /// `src/main.nika` (Part III 13.1).
    pub fn entry(&self) -> PathBuf {
        self.root.join(ENTRY)
    }

    /// Where the generated `Cargo.toml` goes. Under `target/`, because it is
    /// build output: it is regenerated from `nikaia.toml` on every build and
    /// editing it would be editing something the next build overwrites.
    pub fn build_dir(&self) -> PathBuf {
        self.own_dir().join("build")
    }

    /// `target/nikaia/`, or a directory under it of the [`Self::variant`]'s.
    fn own_dir(&self) -> PathBuf {
        let dir = self.root.join("target").join("nikaia");
        match &self.variant {
            Some(variant) => dir.join("variants").join(variant),
            None => dir,
        }
    }

    /// Where the emitted Rust goes - **outside** [`Self::build_dir`], and that
    /// is not tidiness. Cargo lists the files of a path package to decide
    /// whether it is fresh, so a generated source inside the package would make
    /// the package dirty the moment it was written, and every build would
    /// recompile.
    pub fn gen_dir(&self) -> PathBuf {
        self.own_dir().join("gen")
    }

    /// **The switches, as a file Cargo can see** (ADR-021 D5).
    ///
    /// A build switch is not a source, so nothing Cargo watches changes when
    /// one does — and the lowering happens inside the `rustc` wrapper, which
    /// Cargo runs only for a package it thinks is stale. So flipping
    /// `user-parallelism` in `nikaia.toml` left the package fresh and the
    /// binary at the **old** setting, silently: the cache had the switch as a
    /// dimension (D5) and nothing ever got as far as asking it.
    ///
    /// This is the third member of the same family as the two freshness traps
    /// the project build already carries, and it takes the same answer: give
    /// Cargo a file. The driver writes the resolved switches here before
    /// `cargo` runs, and the wrapper puts it into the dependency file beside
    /// the `.nika` sources — so a switch that changed is an input that changed.
    ///
    /// Under `target/` and outside [`Self::build_dir`], for [`Self::gen_dir`]'s
    /// reason: a file *inside* the path package would make it dirty on every
    /// build rather than on a changed one.
    pub fn switches_path(&self) -> PathBuf {
        self.gen_dir().join("switches")
    }

    /// The ledger, in the project root (Part III 13.5).
    pub fn ledger_path(&self) -> PathBuf {
        self.root.join("nikaia.contracts")
    }

    /// The packages this project depends on by path - see [`packages_of`].
    pub fn packages(&self) -> Result<Vec<modules::Dependency>> {
        packages_of(&self.manifest, &self.root, true)
    }

    /// Every package of this build, the entry first - see [`members_of`].
    pub fn members(&self) -> Result<Vec<Member>> {
        members_of(&self.manifest, &self.root)
    }
impl Project {
    /// The program's name for Cargo: in the shared directory its package's
    /// name and [`Self::root_tag`], elsewhere the package's name (#495).
    fn program_name(&self, package: &str) -> String {
        match std::env::var_os("CARGO_TARGET_DIR") {
            Some(_) => package.to_string(),
            None => format!("{package}-{}", self.root_tag()),
        }
    }

    /// What sets this project's program apart in the shared directory: the
    /// start of the hash of where the project is (#495).
    fn root_tag(&self) -> String {
        let mut named = self.root.to_string_lossy().into_owned();
        if let Some(variant) = &self.variant {
            named.push('\u{1f}');
            named.push_str(variant);
        }
        orchestrator::cache::sha256_hex(named.as_bytes())[..16].to_string()
    }

    /// The program a build in the shared directory produced, under the
    /// project's own name in its own `target/nikaia/bin/` (#495): a hard link
    /// where the file system allows one, a copy where not.
    fn own_copy(&self, built: &Path) -> Result<PathBuf> {
        let dir = self.root.join("target").join("nikaia").join("bin");
        std::fs::create_dir_all(&dir)?;
        let name = built
            .file_name()
            .context("a program has a file name")?
            .to_string_lossy()
            .replacen(&format!("-{}", self.root_tag()), "", 1);
        let own = dir.join(name);
        let _ = std::fs::remove_file(&own);
        if std::fs::hard_link(built, &own).is_err() {
            std::fs::copy(built, &own)
                .with_context(|| format!("copying {} to {}", built.display(), own.display()))?;
        }
        Ok(own)
    }
}
/// The switches file the driver wrote, where there is one.
///
/// The wrapper is told the generated-source directory and nothing else
/// (`GEN_DIR_VAR` is the one channel), and the file sits in it — so this is a
/// lookup rather than a second answer to where it lives. Absent means the
/// wrapper was run by hand rather than by a project build, and then there is no
/// switch anybody could flip between two runs.
fn switches_beside(gen_dir: &Path) -> Option<PathBuf> {
    let path = gen_dir.join("switches");
    path.is_file().then_some(path)
}

/// Record one invocation, if anybody asked. A trace that cannot be written is
/// silent: this is a debugging facility, and a build must not fail because the
/// place it was told to write a note to is unwritable.
fn trace(invocation: &Invocation, verdict: &str) {
    use std::io::Write;

    let Some(path) = std::env::var_os(TRACE_VAR) else {
        return;
    };
    let crate_name = invocation.crate_name.as_deref().unwrap_or("-");
    // **And whether this one is Cargo's target-info probe**
    // ([ADR-166](../../docs/specification/adr/adr-166.md) D1), because that is
    // the invocation whose standard input decides whether a build starts at all
    // — and a trace that cannot say so cannot answer the question the trace
    // exists for.
    let verdict = match invocation.is_a_probe() {
        true => &format!("{verdict} (target-info probe)"),
        false => verdict,
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        writeln!(file, "{crate_name} {verdict}").ok();
    }
}
