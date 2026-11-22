//! The build cache: `nikaia.lock` is the record, the key is derived from it.
//!
//! [ADR-021](../../../docs/specification/adr/adr-021.md) decides the shape of
//! this and the reasoning is there; what follows is the part that has to be
//! true in code.
//!
//! * **D2** - the lockfile records everything that *determines* a build, and
//!   only that: input hashes, the toolchain used, and the dependency versions
//!   Cargo resolved. That last one is *recorded* and not keyed, and
//!   [`Lockfile::dependencies`] says why.
//! * **D3** - the compiler's identity (a fingerprint, not a version string) is
//!   in the key. `emit` is a function we edit, so identical input under an
//!   edited emitter is a different build. It is held by [`Cache`], not written
//!   to the lockfile: like D5's switches, it is a build-time fact, and a file
//!   recording it would be rewritten by every rebuild of the compiler (#1945).
//! * **D5** - the build switches and the backend are *choices*, hashed into the key
//!   and never written to the file. A lockfile that records them is rewritten
//!   by every change of switch, for a diff that means nothing.
//! * **D6** - one key per unit. Hashing the whole lockfile would make one
//!   changed asset invalidate the project.
//! * **D7** - every dimension is listed in one place ([`Key::build`]) and
//!   nothing is captured incidentally. Fields are length-prefixed so that no
//!   two different sets of dimensions can hash alike.
//! * **D8** - artifacts are content-addressed under `target/nikaia/cache/`,
//!   with no index of its own: the key is derived, never looked up.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Bumped when the key derivation changes in a way that would otherwise let an
/// artifact from an older scheme be served under a colliding key.
const KEY_DOMAIN: &str = "nikaia-cache-key-v1";

/// The domain of the *sysroot* key ([`Key::sysroot`]).
///
/// A separate domain and not a separate hash function: the two keys live in one
/// module under one [`KeyBuilder`] so that D7's "every dimension is named in one
/// place" still holds, and the domain is what makes a unit key and a sysroot key
/// unable to collide even if their dimensions ever lined up.
const SYSROOT_DOMAIN: &str = "nikaia-sysroot-key-v1";

/// The lockfile format version, so a future reader can refuse rather than
/// misread.
const LOCK_VERSION: u32 = 1;

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in hasher.finalize() {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 15) as usize] as char);
    }
    out
}

/// What was chosen at invocation time (D5).
///
/// These reach the key and never the lockfile. Adding a field here means
/// adding a line to [`Key::build`] - which is the point of keeping them in a
/// struct of their own rather than passing loose strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choices {
    pub build: String,
    pub backend: String,
    /// **The allowlist's own digest**, empty where no list is in effect
    /// ([ADR-310](../../../docs/specification/adr/adr-310.md) D10).
    ///
    /// The list is a file the build read, so it belongs in the key beside the
    /// files it names — and it belongs *here*, with the switches, rather than
    /// among the assets: those are per unit and this binds the whole build
    /// (D6). Changing a line has to invalidate, or a build that stops reading
    /// a file keeps the old answer.
    pub reads: String,
    /// **Every description this build reads at a foreign boundary**, as one
    /// digest, empty where the project describes nothing
    /// ([ADR-rust-boundary-1](../../../docs/specification/adr/adr-rust-boundary-1.md) D5).
    ///
    /// Here for `reads`' reason and with `reads`' polarity: the file binds the
    /// whole build rather than one unit, and it is **hand-edited on purpose** —
    /// D5 says the draft is committed and reviewed like code, and a reviewer's
    /// `crosses = false` has to reach the next build. Without this it did not,
    /// and it failed **open**: a description that *dropped* a claim was seen,
    /// because a refused build records nothing, while one that *added* a claim
    /// hit an entry recorded before the word was there.
    pub describes: String,
    /// **The names the output writes for its files** (0.0.239): the abort
    /// table names each file as it was handed to the compiler (ADR-300 D9),
    /// so `/abs/one.nika` and `one.nika` are two lowerings of one source.
    /// Here and not in the unit's `source`, because that hash is what the
    /// lockfile records, and a path the checkout moves has no place there
    /// (D7).
    pub names: String,
    /// **The committed proofs this build reads** (ADR-270 D19-D22, #448), as
    /// one digest, empty where the project has no `nikaia.proofs`. A lowering
    /// takes its answers from the file, so an edited or regenerated file is
    /// a different build - once a search may differ between machines, the
    /// recorded answer is what decides what is emitted.
    pub proofs: String,
}

impl Choices {
    pub fn new(build: impl Into<String>, backend: impl Into<String>) -> Self {
        Self {
            build: build.into(),
            backend: backend.into(),
            reads: String::new(),
            describes: String::new(),
            names: String::new(),
            proofs: String::new(),
        }
    }

    /// The same, with an allowlist in effect.
    pub fn reading(self, digest: impl Into<String>) -> Self {
        Self {
            reads: digest.into(),
            ..self
        }
    }

    /// The same, with the names the output writes for its files.
    pub fn naming(self, names: impl Into<String>) -> Self {
        Self {
            names: names.into(),
            ..self
        }
    }

    /// The same, with the committed proofs this build reads.
    pub fn proving(self, digest: impl Into<String>) -> Self {
        Self {
            proofs: digest.into(),
            ..self
        }
    }

    /// The same, with the descriptions this build reads at its boundaries.
    pub fn describing(self, digest: impl Into<String>) -> Self {
        Self {
            describes: digest.into(),
            ..self
        }
    }
}

/// What the lockfile records about one translation unit: its inputs, hashed.
///
/// The asset list is what makes the next build's lookup possible at all. A
/// compiler cannot know which external files a unit reads without expanding
/// its macros, so the list is carried over from the previous build - and that
/// is sound because a unit that would read a *different* file had to change
/// its own source to say so, which changes `source` and misses the key.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitRecord {
    /// SHA256 of the unit's own source.
    pub source: String,
    /// Compile-time I/O: asset path -> SHA256 of its contents.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub assets: BTreeMap<String, String>,
}

/// `nikaia.lock` (D2). Committed, and the answer to "does this build the same
/// thing for you as it does for me?".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: u32,
    /// The toolchain actually used - a record, not a constraint (D4). The
    /// constraint lives in the manifest.
    pub toolchain: String,
    /// The versions Cargo resolved, per dependency name (D2).
    ///
    /// **Recorded, and deliberately not in the key.** They belong in the file
    /// because the question it exists to answer - *does this build the same
    /// thing for you as for me?* - is only half answered without them: since
    /// ADR-002 D1 a project build generates a `Cargo.toml` and Cargo resolves a
    /// real graph beside it, and a reviewer or a second machine could not see
    /// any of it. They are **not** dimensions of [`Key`], because Stage 0 lowers
    /// name for name: a dependency bump changes the machine code Cargo
    /// produces, never the Rust this compiler emits, and the envelope a key
    /// holds is exactly that Rust and the ledger. Keying on them would
    /// invalidate every unit's lowering for a change that cannot alter one -
    /// D7's "too much" direction, and the half Cargo's own fingerprinting
    /// already covers (D1).
    ///
    /// A list per name because one name can resolve to two versions in one
    /// graph, and a record that drops one is not the whole answer.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, BTreeSet<String>>,
    /// `BTreeMap`, so the file is byte-identical for identical inputs rather
    /// than ordered by whatever a hash map felt like.
    #[serde(default)]
    pub units: BTreeMap<String, UnitRecord>,
}

impl Lockfile {
    pub fn new(toolchain: impl Into<String>) -> Self {
        Self {
            version: LOCK_VERSION,
            toolchain: toolchain.into(),
            dependencies: BTreeMap::new(),
            units: BTreeMap::new(),
        }
    }

    /// Reads a lockfile, or starts an empty one if the path does not exist.
    ///
    /// A lockfile from a different toolchain or compiler is *kept*, not
    /// discarded: its entries simply cannot match, because the key is built
    /// from the current identity (D3). Throwing it away would lose the asset
    /// lists that make the next lookup possible.
    ///
    /// The compiler's identity is not in the file (D2, D3 as amended for
    /// #1945): it is in every unit's key, which lives in the store under
    /// `target/` (D8). A lock written before that still has a `compiler = ...`
    /// line; it loads, the line is ignored, and `save` writes the file back
    /// without it.
    pub fn load(path: &Path, toolchain: impl Into<String>) -> Result<Self> {
        let toolchain = toolchain.into();
        if !path.exists() {
            return Ok(Self::new(toolchain));
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read lockfile {}", path.display()))?;
        let mut lock: Lockfile = toml::from_str(&text)
            .with_context(|| format!("failed to parse lockfile {}", path.display()))?;
        anyhow::ensure!(
            lock.version == LOCK_VERSION,
            "lockfile {} has version {}, this compiler writes version {LOCK_VERSION}",
            path.display(),
            lock.version
        );
        lock.toolchain = toolchain;
        Ok(lock)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let body = toml::to_string_pretty(self).context("failed to serialise lockfile")?;
        let text = format!(
            "# AUTO-GENERATED by `nikaia build`. Commit this file like a lockfile.\n\
             # It records what determined the build; build-time facts (the switches,\n\
             # backend, the compiler's own identity) are deliberately absent - see\n\
             # ADR-021 D5 and D3: the identity is in every unit's cache key instead.\n\
             # `[dependencies]` is what Cargo resolved, listed per name because one\n\
             # name can resolve twice.\n{body}"
        );
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).ok();
        }
        if std::fs::read_to_string(path).is_ok_and(|held| held == text) {
            return Ok(());
        }
        // Write-then-rename, as `Store::put`: builds side by side read the
        // lockfile of a package they all depend on, and none may read half of
        // one.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(format!(".{}.{n}.partial", std::process::id()));
        let partial = path.with_file_name(name);
        std::fs::write(&partial, text)
            .with_context(|| format!("failed to write lockfile {}", partial.display()))?;
        std::fs::rename(&partial, path).with_context(|| {
            let _ = std::fs::remove_file(&partial);
            format!("failed to write lockfile {}", path.display())
        })
    }
}

/// A cache key for one unit (D6).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key(String);

impl Key {
    /// **The one place every dimension of the key is named** (D7).
    ///
    /// Nothing is hashed that is not listed here, and nothing listed here is
    /// optional. Each field is length-prefixed and tagged, so that no
    /// rearrangement of values can produce the same digest as a different set
    /// of dimensions.
    pub fn build(
        compiler: &str,
        toolchain: &str,
        choices: &Choices,
        unit: &str,
        record: &UnitRecord,
    ) -> Self {
        let mut b = KeyBuilder::new(KEY_DOMAIN);
        b.field("compiler", compiler);
        b.field("toolchain", toolchain);
        b.field("build", &choices.build);
        b.field("backend", &choices.backend);
        // **The allowlist, whether or not anything was read from it** (D7). An
        // empty string is a build with no list, which is a different build from
        // one with an empty list: the first cannot read at all.
        b.field("reads", &choices.reads);
        // **And what the boundary says**
        // ([ADR-rust-boundary-1](../../../docs/specification/adr/adr-rust-boundary-1.md) D5): the
        // descriptions are files this build reads and a person edits, so a
        // change to one has to invalidate — for the reason the line above
        // exists, one boundary over.
        b.field("describes", &choices.describes);
        b.field("names", &choices.names);
        // Only where there is a file: a build without one keys as it did.
        if !choices.proofs.is_empty() {
            b.field("proofs", &choices.proofs);
        }
        b.field("unit", unit);
        b.field("source", &record.source);
        // `BTreeMap` iterates in key order, so the same assets hash the same
        // regardless of the order they were discovered in.
        for (path, hash) in &record.assets {
            b.field("asset-path", path);
            b.field("asset-hash", hash);
        }
        Key(b.finish())
    }

    /// **The one place every dimension of the *sysroot* key is named**
    /// (ADR-002 D4, which is D7 applied to a second store).
    ///
    /// The unit key above answers *what Rust does this `.nika` lower to*. This
    /// one answers *which compiled `std` may this build link*, and the two sets
    /// of dimensions are not the same set:
    ///
    /// * `toolchain` is in both: an rlib is only loadable by the `rustc` that
    ///   wrote it.
    /// * `target` is in, and `user_parallelism` is deliberately **not**: the
    ///   switch reaches `std` as a runtime value, never as a `cfg`, so there is
    ///   one compiled `std` per machine rather than one per build switch
    ///   (ADR-002 D4).
    /// * `codegen` is in - the rendered `[build.<target>]` table and the panic
    ///   strategy. It is what makes a hardware-specific build a *dimension*
    ///   rather than a reason to rebuild: `std`'s code runs inside every user
    ///   program, so it is the one place where optimising for the machine in
    ///   front of you pays, and two such builds have to be able to coexist.
    ///
    /// What is **out**, and why each is D7's "too much" direction here:
    ///
    /// * **The `std` sources' content hash.** What this key names is a
    ///   directory, and *inside* it Cargo's own fingerprinting decides what is
    ///   stale (D1). Hashing the tree as well would start a fresh directory for
    ///   every edit to `crates/nikaia-std/src/fs.rs` and rebuild everything
    ///   `std` depends on, to protect against a staleness Cargo has already
    ///   ruled out.
    /// * **The project**, its path and its resolved dependency versions. None
    ///   of them can change a byte of the compiled `std`, and any of them would
    ///   turn the one shared entry into a per-project one - which is the entire
    ///   thing this cache exists to stop.
    /// * **The lowering backend.** It changes what a `.nika` file lowers to,
    ///   and `std`'s Rust is not lowered by this build at all.
    /// * **The compiler's own identity**, which the unit key has (D3). `std`'s
    ///   Rust is committed already lowered, so this compiler writes not a byte
    ///   of what is compiled here; with it in, every edit to the compiler
    ///   started a fresh directory and compiled `std` and everything under it
    ///   again. The program's own crates in the directory are the ones the
    ///   compiler does write, and they go stale through the switches file
    ///   instead (ADR-002 D4).
    pub fn sysroot(
        toolchain: &str,
        target: &str,
        codegen: &str,
        features: &str,
        cpu: &str,
    ) -> Self {
        let mut b = KeyBuilder::new(SYSROOT_DOMAIN);
        b.field("toolchain", toolchain);
        b.field("target", target);
        b.field("codegen", codegen);
        // **A switch that changes what `std` itself contains is a dimension of
        // the compiled `std`** ([ADR-021](../../../docs/specification/adr/adr-021.md)
        // D7, [ADR-281](../../../docs/specification/adr/adr-281.md) D36): the
        // re-entrancy check lives in that crate, so without this the second
        // project on the machine links the first one's answer — a guarantee
        // declined in one manifest and shipped from another's cache.
        b.field("features", features);
        // **The CPU level `--cpu` named** (ADR-002 D6): every crate built for
        // the target, `std` among them, may use its instructions. Empty for
        // the baseline, and then not hashed at all, so a build that names no
        // level keeps the entry it had; for `native` the CPU found, never the
        // word.
        if !cpu.is_empty() {
            b.field("cpu", cpu);
        }
        Key(b.finish())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

struct KeyBuilder {
    hasher: Sha256,
}

impl KeyBuilder {
    fn new(domain: &str) -> Self {
        let mut b = KeyBuilder {
            hasher: Sha256::new(),
        };
        b.field("domain", domain);
        b
    }

    fn field(&mut self, name: &str, value: &str) {
        // Length prefixes are what make this unambiguous: without them
        // ("ab", "c") and ("a", "bc") would hash identically.
        self.hasher.update((name.len() as u64).to_le_bytes());
        self.hasher.update(name.as_bytes());
        self.hasher.update((value.len() as u64).to_le_bytes());
        self.hasher.update(value.as_bytes());
    }

    fn finish(self) -> String {
        self.hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

/// Everything one cached build produced, under names the caller chooses.
///
/// A build makes more than the emitted Rust - ADR-020's contract ledger is an
/// output too, and a later stage may add more. Storing them under separate keys
/// would let a "hit" be half a build: the Rust present, the ledger missing, and
/// nothing to notice it. One envelope per key keeps a hit meaning what it says.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifacts(BTreeMap<String, String>);

impl Artifacts {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with(mut self, name: impl Into<String>, content: impl Into<String>) -> Self {
        self.0.insert(name.into(), content.into());
        self
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// Whether every name the caller needs is present. A build that asks for
    /// more than an older entry holds must miss rather than proceed with a
    /// gap - which is what makes adding an artifact a safe change.
    pub fn has_all(&self, names: &[&str]) -> bool {
        names.iter().all(|name| self.0.contains_key(*name))
    }
}

/// The content-addressed artifact store (D8), rooted at `target/nikaia/cache/`.
///
/// No index: an entry's path is a function of its key, so the store cannot
/// disagree with the lockfile about what it holds.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Store { root: root.into() }
    }

    fn path_for(&self, key: &Key) -> PathBuf {
        // Two-level fan-out, so a large store does not become one directory
        // with a hundred thousand entries in it.
        let k = key.as_str();
        self.root.join(&k[..2]).join(&k[2..])
    }

    pub fn get(&self, key: &Key) -> Option<String> {
        std::fs::read_to_string(self.path_for(key)).ok()
    }

    pub fn put(&self, key: &Key, artifact: &str) -> Result<()> {
        let path = self.path_for(key);
        let dir = path.parent().expect("store paths always have a parent");
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create cache directory {}", dir.display()))?;
        // Write-then-rename: a reader never sees a half-written artifact, and
        // two builds racing on the same key both end up with a complete file.
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        std::fs::write(&tmp, artifact)
            .with_context(|| format!("failed to write cache entry {}", tmp.display()))?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("failed to commit cache entry {}", path.display()))
    }
}

/// The lockfile and the store, used together.
///
/// The division of labour is ADR-021 D10's: this decides *whether to start*.
/// It is consulted before any work and answers with an artifact or with
/// nothing. It is not the contract ledger, which can only be compared after
/// inference has run.
#[derive(Debug)]
pub struct Cache {
    lock_path: PathBuf,
    store: Store,
    lock: Lockfile,
    /// The compiler's identity (D3): a dimension of every unit's key, and
    /// deliberately not a line of the committed lockfile (D2, #1945).
    compiler: String,
}

impl Cache {
    pub fn open(
        lock_path: impl Into<PathBuf>,
        store_root: impl Into<PathBuf>,
        toolchain: &str,
        compiler: &str,
    ) -> Result<Self> {
        let lock_path = lock_path.into();
        let lock = Lockfile::load(&lock_path, toolchain)?;
        Ok(Cache {
            lock_path,
            store: Store::new(store_root),
            lock,
            compiler: compiler.to_string(),
        })
    }

    pub fn lockfile(&self) -> &Lockfile {
        &self.lock
    }

    /// Hashes the assets this unit read on the previous build, as they are on
    /// disk *now*. A recorded asset that has since disappeared is a miss, not
    /// an error: the unit is simply rebuilt, and the build reports the real
    /// problem if the file is still needed.
    fn current_record(&self, unit: &str, source: &str, asset_root: &Path) -> Option<UnitRecord> {
        let previous = self.lock.units.get(unit);
        let mut assets = BTreeMap::new();
        if let Some(previous) = previous {
            for path in previous.assets.keys() {
                let bytes = std::fs::read(asset_root.join(path)).ok()?;
                assets.insert(path.clone(), sha256_hex(&bytes));
            }
        }
        Some(UnitRecord {
            source: sha256_hex(source.as_bytes()),
            assets,
        })
    }

    /// The skip decision. `Some(artifacts)` means nothing has to be built -
    /// not the lowering, and not the checks that ran before it, because only a
    /// build that passed them was ever recorded (D13).
    ///
    /// A stored entry that cannot be decoded reads as a miss: it was written by
    /// a different version of this format, and rebuilding is always the right
    /// answer to not understanding what is there.
    pub fn lookup(
        &self,
        unit: &str,
        source: &str,
        choices: &Choices,
        asset_root: &Path,
    ) -> Option<Artifacts> {
        let record = self.current_record(unit, source, asset_root)?;
        let key = Key::build(&self.compiler, &self.lock.toolchain, choices, unit, &record);
        serde_json::from_str(&self.store.get(&key)?).ok()
    }

    /// Records a freshly built unit: its inputs into the lockfile, everything
    /// it produced into the store. `assets` is what the build actually read -
    /// an empty map is correct for a unit that read nothing.
    ///
    /// Only ever called for a build that succeeded, which is what lets a later
    /// hit skip the checks as well as the work (D13).
    pub fn record(
        &mut self,
        unit: &str,
        source: &str,
        assets: BTreeMap<String, String>,
        choices: &Choices,
        artifacts: &Artifacts,
    ) -> Result<()> {
        let record = UnitRecord {
            source: sha256_hex(source.as_bytes()),
            assets,
        };
        let key = Key::build(&self.compiler, &self.lock.toolchain, choices, unit, &record);
        let encoded = serde_json::to_string(artifacts).context("failed to encode artifacts")?;
        self.store.put(&key, &encoded)?;
        self.lock.units.insert(unit.to_string(), record);
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        self.lock.save(&self.lock_path)
    }

    /// What the recorded build of `unit` read while it was built, relative to
    /// its asset root: the files besides its sources a run that skips the
    /// build checks (#2421).
    pub fn assets_of(&self, unit: &str) -> Vec<String> {
        self.lock
            .units
            .get(unit)
            .map(|record| record.assets.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// Where the lockfile and the store live for a given input.
///
/// Caching is on by default - it is the difference in feel between a Nikaia
/// build and a Rust one, and a default nobody types is not that. Being on by
/// default is what makes *where* the files go a real question rather than a
/// detail: nothing may be written next to a source file the user did not ask
/// us to write next to.
///
/// So there are two shapes, and the manifest decides which:
///
/// * **In a project** (`nikaia.toml` found by walking up, as Cargo does) the
///   lock is the committed record ADR-021 D2 describes, and the store sits in
///   `target/` where build output belongs.
/// * **Outside one** the invocation is a one-off transformation. A *committed*
///   lockfile would be meaningless there, because there is no project to commit
///   it to, so the input record moves into the user's cache directory next to
///   the store and the source tree is left completely untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// The directory the unit is named relative to, and assets are resolved
    /// against.
    pub root: PathBuf,
    pub lock: PathBuf,
    pub store: PathBuf,
    /// Whether a `nikaia.toml` was found. False means nothing is written into
    /// the source tree.
    pub in_project: bool,
}

impl Layout {
    /// `NIKAIA_CACHE_DIR` overrides the user cache location. It moves *where*
    /// artifacts are kept and never what they are keyed by, so it is not a
    /// dimension in D7's sense - and tests need somewhere that is not the
    /// developer's real cache.
    ///
    /// Public because the compiled-`std` cache of ADR-002 D4 lives here too.
    /// That cache is machine-wide by definition - the whole point is that a
    /// second project does not rebuild it - so it cannot live under any one
    /// project's `target/`, and this is the directory D11 already sanctions for
    /// state that outlives one project.
    pub fn user_cache_dir() -> PathBuf {
        for var in ["NIKAIA_CACHE_DIR", "XDG_CACHE_HOME"] {
            if let Some(dir) = std::env::var_os(var).filter(|v| !v.is_empty()) {
                let dir = PathBuf::from(dir);
                return if var == "NIKAIA_CACHE_DIR" {
                    dir
                } else {
                    dir.join("nikaia")
                };
            }
        }
        if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
            return PathBuf::from(home).join(".cache").join("nikaia");
        }
        std::env::temp_dir().join("nikaia-cache")
    }

    /// **Where Cargo, `rustc` and every tool rustup starts are started**
    /// ([ADR-325](../../../docs/specification/adr/adr-325.md) D1): a directory
    /// of the compiler's own, under the user cache and outside every project.
    ///
    /// Cargo reads `.cargo/config.toml` from the directory it is started in
    /// and every one above it, and rustup reads `rust-toolchain.toml` the same
    /// way. Started inside a project, both would take orders from files the
    /// project carries, such as a linker to run. Started here, they read only
    /// the user's own: `CARGO_HOME`, the environment and rustup's default
    /// (D3). Every path handed to them is therefore absolute.
    pub fn where_tools_start() -> PathBuf {
        let dir = Layout::user_cache_dir().join("tools");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    pub fn resolve(input: &Path) -> Layout {
        let start = Self::start_of(input);
        for dir in start.ancestors() {
            if dir.join("nikaia.toml").is_file() {
                return Layout {
                    root: dir.to_path_buf(),
                    lock: dir.join("nikaia.lock"),
                    store: dir.join("target").join("nikaia").join("cache"),
                    in_project: true,
                };
            }
        }
        Self::outside(start)
    }

    /// **The file as one outside every project**, whatever is above it: a
    /// test program (ADR-269 D22, #972) is a program of its own, built by
    /// `nikaia test` in a project of its own, so the project around it keeps
    /// no record of it and lends it no committed ledger.
    pub fn alone(input: &Path) -> Layout {
        Self::outside(Self::start_of(input))
    }

    /// The directory `input` sits in, canonicalised where it exists.
    fn start_of(input: &Path) -> PathBuf {
        let start = input
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        // Canonicalised so that `./src/../src` and `src` are one directory, and
        // so the record name below is stable. A failure here (the path does not
        // exist yet) is not fatal - the uncanonicalised path still works.
        start.canonicalize().unwrap_or(start)
    }

    /// The layout of a file outside every project, from its directory.
    fn outside(start: PathBuf) -> Layout {
        let base = Self::user_cache_dir();
        // One record per directory, named by a hash of the directory rather
        // than placed in it. Two projects that both have a `main.nika` must not
        // share a record - but they may share *store* entries, because the key
        // is content-addressed and deliberately holds no path (D7).
        let record = format!("{}.lock", sha256_hex(start.as_os_str().as_encoded_bytes()));
        Layout {
            root: start,
            lock: base.join("records").join(record),
            store: base.join("cache"),
            in_project: false,
        }
    }

    /// The unit's name: its path below the root, with forward slashes so that
    /// a lockfile written on Windows and one written on Linux agree. Never an
    /// absolute path - that is D7's first failure direction, a key that moves
    /// with the checkout and therefore never hits.
    pub fn unit_name(&self, input: &Path) -> String {
        let canonical = input.canonicalize();
        let path = canonical.as_deref().unwrap_or(input);
        match path.strip_prefix(&self.root) {
            Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
            // Outside the root entirely: fall back to the file name rather than
            // leaking an absolute path into the key.
            Err(_) => path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
        }
    }
}

/// **The user cache is bounded by its size, and by nothing else** (#724).
///
/// Eight GiB unless `NIKAIA_CACHE_LIMIT` says otherwise. Every kind of entry
/// is held to the one bound together: a compiled-`std` tree (around 240 MB, and
/// every rebuild of this compiler starts one), a build-time or run bundle, a
/// grammar parser, a file run on its own. Unbounded, the cache reached 15 GB and
/// nearly filled a disk.
pub const DEFAULT_LIMIT: u64 = 8 << 30;

/// The variable that sets the bound: bytes, or a number followed by `K`, `M`,
/// `G` or `T` (powers of 1024). `0` is no bound.
pub const LIMIT_VAR: &str = "NIKAIA_CACHE_LIMIT";

/// The bound this process works to, and a warning when the variable says
/// something that is not a size. A bad value is the default, never a failed
/// build (ADR-021 D12).
pub fn limit() -> (u64, Option<String>) {
    match std::env::var(LIMIT_VAR) {
        Err(_) => (DEFAULT_LIMIT, None),
        Ok(text) if text.trim().is_empty() => (DEFAULT_LIMIT, None),
        Ok(text) => match parse_size(&text) {
            Some(bytes) => (bytes, None),
            None => (
                DEFAULT_LIMIT,
                Some(format!(
                    "warning: {LIMIT_VAR}={text:?} is not a size (bytes, or a number \
                     followed by K, M, G or T); the cache is held to the default {}",
                    show_size(DEFAULT_LIMIT)
                )),
            ),
        },
    }
}

/// `8G`, `500M`, `1024`: a size in bytes, the suffixes powers of 1024.
pub fn parse_size(text: &str) -> Option<u64> {
    let text = text.trim();
    let text = text
        .strip_suffix(['B', 'b'])
        .filter(|rest| !rest.is_empty())
        .unwrap_or(text);
    let (digits, shift) = match text.chars().last()?.to_ascii_uppercase() {
        'K' => (&text[..text.len() - 1], 10),
        'M' => (&text[..text.len() - 1], 20),
        'G' => (&text[..text.len() - 1], 30),
        'T' => (&text[..text.len() - 1], 40),
        _ => (text, 0),
    };
    let number: u64 = digits.trim().parse().ok()?;
    number.checked_mul(1u64 << shift)
}

/// A size as a person reads it: `1.5 GiB`, `312 MiB`, `40 KiB`, `12 B`.
pub fn show_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if value < 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.0} {}", UNITS[unit])
    }
}

/// The lock files of the entries this process uses, held open until it ends:
/// a shared lock on each tells every other process's [`prune`] that the entry
/// is in use.
static HELD: std::sync::Mutex<Vec<std::fs::File>> = std::sync::Mutex::new(Vec::new());

/// Whether this process started an entry, and so whether [`settle`] has work.
static ADDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// **Mark an entry used**: what every user of a user-cache entry calls before it
/// reads or writes the entry.
///
/// The stamp's time is the entry's last use - written by this compiler, not the
/// file system's access time, which most mounts do not keep. The entry's lock is
/// taken shared and held until the process ends, so no [`prune`] removes it under
/// this build. An entry that did not exist yet makes the next [`settle`] prune.
pub fn use_entry(entry: &Path) {
    if !entry.join(USED).exists() {
        ADDED.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    touch(entry);
    hold(entry);
}

/// **Take the entry's lock shared, and keep it.**
///
/// A [`prune`] that holds the entry exclusively is removing it: this waits
/// for it, and when the entry it held is gone (its lock file no longer the one
/// at the path), makes the entry again and holds that. Taken without waiting,
/// a build that met a prune went on unheld in a directory being removed, and
/// a run of the same file at the same moment found its files gone (#1128).
fn hold(entry: &Path) {
    for _ in 0..8 {
        let Ok(file) = open_lock(entry) else {
            touch(entry);
            continue;
        };
        if !lock::shared(&file) && !lock::shared_waiting(&file) {
            return;
        }
        if !lock::still_at(&file, &entry.join(LOCK)) {
            touch(entry);
            continue;
        }
        if let Ok(mut held) = HELD.lock() {
            held.push(file);
        }
        return;
    }
}

/// **Hold the lock file `path` alone until the result is dropped**: what a
/// step that reads and writes several files of a directory takes, so builds of
/// the same thing at the same moment take turns and the later one finds the
/// earlier one's result (#1128). `None` where the lock cannot be had; the
/// caller then goes on unlocked, as before.
pub fn alone(path: &Path) -> Option<std::fs::File> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .ok()?;
    lock::exclusive_waiting(&file).then_some(file)
}

fn open_lock(entry: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(entry.join(LOCK))
}

/// **Prune once a build has added an entry**: what a build calls when Cargo is
/// done, and the compiler once more before it exits. Nothing happens when this
/// process started no entry, so a build that only reused the cache does not
/// walk it.
pub fn settle() {
    if !ADDED.swap(false, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let (limit, warning) = limit();
    if let Some(warning) = warning {
        eprintln!("{warning}");
    }
    prune(&Layout::user_cache_dir(), limit);
}

/// What one [`prune`] pass did.
#[derive(Debug, Default)]
pub struct Pruned {
    /// Each entry removed and its size, the least recently used first.
    pub removed: Vec<(PathBuf, u64)>,
    /// What the entries left come to.
    pub size: u64,
}

/// **Remove the entries used longest ago until the cache is within `limit`.**
///
/// Only the last use counts: an entry goes because others were used after it,
/// however young it is, and a month-old entry that is still used stays. `0` is
/// no limit.
///
/// An entry is a directory two levels below the root (`rlib/<key>`,
/// `run-bundle/<key>`, ...) that carries the stamp [`use_entry`] writes; what has
/// no stamp is not this pass's to remove. Never removed: an entry a running build
/// holds - this compiler's shared lock ([`use_entry`]), which includes the one
/// this process just used, or Cargo's lock on a target directory inside it.
///
/// **Failure here is never a failed build.** A removed entry is a later cache
/// miss; an entry that cannot be removed is a fuller disk (ADR-021 D12).
pub fn prune(root: &Path, limit: u64) -> Pruned {
    let mut entries = entries(root);
    let mut pruned = Pruned {
        removed: Vec::new(),
        size: entries.iter().map(|entry| entry.size).sum(),
    };
    if limit == 0 || pruned.size <= limit {
        return pruned;
    }
    // Used longest ago first.
    entries.sort_by_key(|entry| entry.used);
    for entry in entries {
        if pruned.size <= limit {
            break;
        }
        let Ok(file) = open_lock(&entry.path) else {
            continue;
        };
        // Held exclusively while it is removed, so a build that starts using it
        // now waits and then finds it gone: a miss.
        if lock::recently_used(entry.used) || !lock::exclusive(&file) || cargo_holds(&entry.path) {
            continue;
        }
        if std::fs::remove_dir_all(&entry.path).is_ok() {
            pruned.size = pruned.size.saturating_sub(entry.size);
            pruned.removed.push((entry.path, entry.size));
        }
    }
    pruned
}

struct Entry {
    path: PathBuf,
    used: std::time::SystemTime,
    size: u64,
}

/// Every entry of the cache: the stamped directories two levels down.
fn entries(root: &Path) -> Vec<Entry> {
    let dirs = |dir: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
            .map(|entry| entry.path())
            .collect()
    };
    dirs(root)
        .iter()
        .flat_map(|kind| dirs(kind))
        .filter(|path| path.join(USED).is_file())
        .map(|path| Entry {
            used: used_at(&path),
            size: size_of(&path),
            path,
        })
        .collect()
}

/// What the files under a directory come to. Links are not followed, and a
/// file Cargo hard-linked into a second place (`deps/x` and `x`) is counted once.
fn size_of(dir: &Path) -> u64 {
    let mut seen = BTreeSet::new();
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                stack.push(entry.path());
            } else if first_link(&meta, &mut seen) {
                total += meta.len();
            }
        }
    }
    total
}

/// Whether this is the first name of its file met in this walk.
#[cfg(unix)]
fn first_link(meta: &std::fs::Metadata, seen: &mut BTreeSet<(u64, u64)>) -> bool {
    use std::os::unix::fs::MetadataExt;
    meta.nlink() < 2 || seen.insert((meta.dev(), meta.ino()))
}

#[cfg(not(unix))]
fn first_link(_: &std::fs::Metadata, _: &mut BTreeSet<(u64, u64)>) -> bool {
    true
}

/// Whether Cargo is building in a target directory inside the entry: it holds
/// `.cargo-lock` of the profile directory (`debug/`, `<triple>/release/`, ...)
/// locked while it runs.
fn cargo_holds(entry: &Path) -> bool {
    let mut stack = vec![(entry.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        for item in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = item.path();
            if item.file_name() == ".cargo-lock" {
                let busy = std::fs::File::open(&path).is_ok_and(|file| !lock::exclusive(&file));
                if busy {
                    return true;
                }
            } else if depth < 4 && item.file_type().is_ok_and(|t| t.is_dir()) {
                stack.push((path, depth + 1));
            }
        }
    }
    false
}

/// Whether `file` is now locked exclusively by this process, taken without
/// waiting (`flock`; without it, always): a test place kept from one run to
/// the next is used by one run at a time (#1750).
pub fn lock_exclusive(file: &std::fs::File) -> bool {
    lock::exclusive(file)
}

/// Advisory locks, taken without waiting: `flock`, the call Cargo locks its
/// target directory with. A lock is released when its file is closed.
#[cfg(unix)]
mod lock {
    use rustix::fs::{FlockOperation, flock};

    pub fn shared(file: &std::fs::File) -> bool {
        flock(file, FlockOperation::NonBlockingLockShared).is_ok()
    }

    pub fn exclusive(file: &std::fs::File) -> bool {
        flock(file, FlockOperation::NonBlockingLockExclusive).is_ok()
    }

    /// Shared, waiting until no exclusive holder is left.
    pub fn shared_waiting(file: &std::fs::File) -> bool {
        flock(file, FlockOperation::LockShared).is_ok()
    }

    /// Exclusive, waiting until every other holder is gone.
    pub fn exclusive_waiting(file: &std::fs::File) -> bool {
        flock(file, FlockOperation::LockExclusive).is_ok()
    }

    /// Whether `path` still names `file`: a prune that removed the entry
    /// took its lock file with it.
    pub fn still_at(file: &std::fs::File, path: &std::path::Path) -> bool {
        use std::os::unix::fs::MetadataExt;
        match (file.metadata(), std::fs::metadata(path)) {
            (Ok(held), Ok(named)) => held.dev() == named.dev() && held.ino() == named.ino(),
            _ => false,
        }
    }

    /// With locks, a lock says whether an entry is in use.
    pub fn recently_used(_: std::time::SystemTime) -> bool {
        false
    }
}

/// Without `flock`, an entry used within the hour counts as held: a build
/// that has not touched its entry for that long is not one that is running.
#[cfg(not(unix))]
mod lock {
    pub fn shared(_: &std::fs::File) -> bool {
        true
    }

    pub fn exclusive(_: &std::fs::File) -> bool {
        true
    }

    pub fn shared_waiting(_: &std::fs::File) -> bool {
        true
    }

    pub fn exclusive_waiting(_: &std::fs::File) -> bool {
        true
    }

    pub fn still_at(_: &std::fs::File, _: &std::path::Path) -> bool {
        true
    }

    pub fn recently_used(used: std::time::SystemTime) -> bool {
        used.elapsed()
            .is_ok_and(|idle| idle < std::time::Duration::from_secs(60 * 60))
    }
}

/// **`nikaia cache clean`**: empty the user cache by hand. Everything in it is
/// rebuilt on demand. Returns the directory that was emptied.
pub fn clean() -> std::io::Result<PathBuf> {
    let dir = Layout::user_cache_dir();
    clean_dir(&dir)?;
    Ok(dir)
}

fn clean_dir(dir: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Say that a cache directory is in use now.
pub fn touch(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(dir.join(USED), b"");
}

/// When a cache directory was last said to be in use.
fn used_at(dir: &Path) -> std::time::SystemTime {
    std::fs::metadata(dir.join(USED))
        .or_else(|_| std::fs::metadata(dir))
        .and_then(|meta| meta.modified())
        .unwrap_or(std::time::UNIX_EPOCH)
}

/// The stamp `touch` writes. A file rather than the directory's own mtime,
/// because Cargo writes into subdirectories and leaves the top alone.
const USED: &str = ".nikaia-used";

/// The file an entry's users lock shared and [`prune`] locks exclusively.
const LOCK: &str = ".nikaia-lock";

#[cfg(test)]
mod tests {
    use super::*;

    fn choices() -> Choices {
        Choices::new("x86_64-linux/auto", "rust")
    }

    fn emitted() -> Artifacts {
        Artifacts::new().with("rust", "EMITTED")
    }

    fn record(source: &str) -> UnitRecord {
        UnitRecord {
            source: sha256_hex(source.as_bytes()),
            assets: BTreeMap::new(),
        }
    }

    fn key_with(compiler: &str, toolchain: &str, c: &Choices, unit: &str, src: &str) -> Key {
        Key::build(compiler, toolchain, c, unit, &record(src))
    }

    #[test]
    fn identical_inputs_produce_identical_keys() {
        let a = key_with("0.1.0", "rustc-x", &choices(), "a.nika", "fn main() {}");
        let b = key_with("0.1.0", "rustc-x", &choices(), "a.nika", "fn main() {}");
        assert_eq!(a, b);
    }

    /// D5. Without this the cache would hand one build's artifact to the
    /// other, and `the_switches_agree_on_every_example` would compare an
    /// artifact against itself.
    #[test]
    fn the_switches_change_the_key() {
        let sequential = Choices::new("x86_64-linux/0", "rust");
        let parallel = Choices::new("x86_64-linux/auto", "rust");
        assert_ne!(
            key_with("0.1.0", "rustc-x", &sequential, "a.nika", "src"),
            key_with("0.1.0", "rustc-x", &parallel, "a.nika", "src"),
        );
    }

    /// D3. The emitter is a function we edit; a changed compiler is a changed
    /// build even when nothing else moved.
    #[test]
    fn the_compiler_version_changes_the_key() {
        assert_ne!(
            key_with("0.1.0", "rustc-x", &choices(), "a.nika", "src"),
            key_with("0.2.0", "rustc-x", &choices(), "a.nika", "src"),
        );
    }

    #[test]
    fn the_toolchain_changes_the_key() {
        assert_ne!(
            key_with("0.1.0", "rustc-x", &choices(), "a.nika", "src"),
            key_with("0.1.0", "rustc-y", &choices(), "a.nika", "src"),
        );
    }

    #[test]
    fn the_backend_changes_the_key() {
        assert_ne!(
            key_with(
                "0.1.0",
                "rustc-x",
                &Choices::new("x86_64-linux/auto", "rust"),
                "a",
                "s"
            ),
            key_with(
                "0.1.0",
                "rustc-x",
                &Choices::new("x86_64-linux/auto", "interpreter"),
                "a",
                "s"
            ),
        );
    }

    /// D7. Without length prefixes, moving a character from one field to the
    /// next would leave the digest unchanged.
    #[test]
    fn fields_cannot_run_into_one_another() {
        let ab = Choices::new("ab", "c");
        let a_bc = Choices::new("a", "bc");
        assert_ne!(
            key_with("0.1.0", "t", &ab, "u", "s"),
            key_with("0.1.0", "t", &a_bc, "u", "s"),
        );
    }

    /// D6. The key covers one unit's inputs, so a sibling's change cannot
    /// invalidate it.
    #[test]
    fn a_sibling_unit_does_not_affect_this_key() {
        let mut lock = Lockfile::new("rustc-x");
        lock.units.insert("a.nika".into(), record("a"));
        let before = Key::build(
            "0.1.0",
            &lock.toolchain,
            &choices(),
            "a.nika",
            lock.units.get("a.nika").unwrap(),
        );
        lock.units.insert("b.nika".into(), record("b changed"));
        let after = Key::build(
            "0.1.0",
            &lock.toolchain,
            &choices(),
            "a.nika",
            lock.units.get("a.nika").unwrap(),
        );
        assert_eq!(before, after);
    }

    /// Assets are hashed on content, and the order they were discovered in is
    /// not a dimension.
    #[test]
    fn asset_order_is_not_a_dimension_but_content_is() {
        let mut one = UnitRecord {
            source: sha256_hex(b"s"),
            assets: BTreeMap::new(),
        };
        one.assets.insert("x.sql".into(), sha256_hex(b"X"));
        one.assets.insert("y.sql".into(), sha256_hex(b"Y"));
        let mut two = UnitRecord {
            source: sha256_hex(b"s"),
            assets: BTreeMap::new(),
        };
        two.assets.insert("y.sql".into(), sha256_hex(b"Y"));
        two.assets.insert("x.sql".into(), sha256_hex(b"X"));
        assert_eq!(
            Key::build("0.1.0", "t", &choices(), "u", &one),
            Key::build("0.1.0", "t", &choices(), "u", &two),
        );

        two.assets.insert("y.sql".into(), sha256_hex(b"Y changed"));
        assert_ne!(
            Key::build("0.1.0", "t", &choices(), "u", &one),
            Key::build("0.1.0", "t", &choices(), "u", &two),
        );
    }

    /// Caching is on by default, so this is the promise that makes it
    /// acceptable: outside a `nikaia.toml` project, nothing goes into the
    /// directory the source lives in.
    ///
    /// Asserted as "not under the input directory" rather than against a
    /// specific fallback path, so the test neither depends on the environment
    /// nor has to mutate it - `set_var` is process-global and these tests run
    /// in parallel threads.
    #[test]
    fn outside_a_project_nothing_is_written_beside_the_source() {
        let dir = scratch("layout-loose");
        let input = dir.join("main.nika");
        std::fs::write(&input, "fn main() {}").unwrap();

        let layout = Layout::resolve(&input);
        assert!(!layout.in_project);
        let canonical = dir.canonicalize().unwrap();
        assert!(
            !layout.lock.starts_with(&canonical),
            "the lock must not land beside the source: {}",
            layout.lock.display()
        );
        assert!(
            !layout.store.starts_with(&canonical),
            "nor the store: {}",
            layout.store.display()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// In a project the lock is the committed record at the root, found by
    /// walking up from the input the way Cargo finds `Cargo.toml`.
    #[test]
    fn a_manifest_makes_the_root_and_names_the_unit_below_it() {
        let dir = scratch("layout-project");
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(dir.join("nikaia.toml"), "[package]\nname = \"demo\"\n").unwrap();
        let input = src.join("main.nika");
        std::fs::write(&input, "fn main() {}").unwrap();

        let layout = Layout::resolve(&input);
        let root = dir.canonicalize().unwrap();
        assert!(layout.in_project);
        assert_eq!(layout.root, root);
        assert_eq!(layout.lock, root.join("nikaia.lock"));
        assert_eq!(layout.store, root.join("target/nikaia/cache"));
        // Relative to the root, so the key does not move with the checkout.
        assert_eq!(layout.unit_name(&input), "src/main.nika");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A store that cannot be created reports an error rather than panicking,
    /// so the caller can carry on with a slower build. Enforced with a *file*
    /// where a directory would have to go, which even root cannot write into.
    #[test]
    fn an_unusable_store_reports_instead_of_panicking() {
        let dir = scratch("store-blocked");
        let blocker = dir.join("blocker");
        std::fs::write(&blocker, "not a directory").unwrap();

        let store = Store::new(blocker.join("cache"));
        let key = Key::build("0.1.0", "t", &choices(), "a.nika", &record("src"));
        assert!(store.get(&key).is_none(), "a missing entry reads as a miss");
        assert!(
            store.put(&key, "EMITTED").is_err(),
            "an unusable store must surface an error, not unwind"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    fn scratch(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "nikaia-cache-{name}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_recorded_unit_is_found_again_and_a_changed_one_is_not() {
        let dir = scratch("roundtrip");
        let mut cache =
            Cache::open(dir.join("nikaia.lock"), dir.join("store"), "t", "0.1.0").unwrap();

        assert!(
            cache
                .lookup("a.nika", "fn main() {}", &choices(), &dir)
                .is_none()
        );

        cache
            .record(
                "a.nika",
                "fn main() {}",
                BTreeMap::new(),
                &choices(),
                &emitted(),
            )
            .unwrap();

        assert_eq!(
            cache.lookup("a.nika", "fn main() {}", &choices(), &dir),
            Some(emitted())
        );
        // A changed source is a different unit as far as the key is concerned.
        assert!(
            cache
                .lookup("a.nika", "fn other() {}", &choices(), &dir)
                .is_none()
        );
        // As is the same source under different switches.
        assert!(
            cache
                .lookup(
                    "a.nika",
                    "fn main() {}",
                    &Choices::new("x86_64-linux/0", "rust"),
                    &dir
                )
                .is_none()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// D5, on disk: the switches and backend must not appear in the committed
    /// file, or changing one would rewrite it for nothing.
    #[test]
    fn the_lockfile_records_inputs_and_not_choices() {
        let dir = scratch("lockfile");
        let path = dir.join("nikaia.lock");
        let mut cache =
            Cache::open(&path, dir.join("store"), "rustc-x", "compiler-identity").unwrap();
        cache
            .record(
                "a.nika",
                "fn main() {}",
                BTreeMap::new(),
                &choices(),
                &emitted(),
            )
            .unwrap();
        cache.save().unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("a.nika"), "the unit is recorded:\n{text}");
        assert!(
            text.contains("rustc-x"),
            "the toolchain is recorded:\n{text}"
        );
        assert!(
            !text.contains("compiler-identity") && !text.contains("compiler ="),
            "the compiler's identity is in the key, not the file (D2, D3, #1945):\n{text}"
        );
        assert!(
            !text.contains("x86_64-linux"),
            "the build must not be:\n{text}"
        );

        // And it round-trips.
        let reloaded = Lockfile::load(&path, "rustc-x").unwrap();
        assert_eq!(reloaded.units, cache.lockfile().units);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// D2's resolved dependency versions: written to the file, absent from the
    /// key - and both halves are the decision.
    ///
    /// **Recorded**, because a lockfile that cannot say which `regex` was
    /// linked answers *does this build the same thing for you as for me* only
    /// about the Nikaia half. **Not keyed**, because Stage 0 lowers name for
    /// name: a dependency bump changes the machine code Cargo produces, never
    /// the Rust in the envelope the key holds. Keying on it would invalidate
    /// every unit's lowering for a change that cannot alter one, which is D7's
    /// "too much" direction - and the assertion below is what would fail if
    /// someone later threaded them into [`Key::build`] for symmetry.
    #[test]
    fn resolved_dependency_versions_are_recorded_and_not_keyed() {
        let dir = scratch("lockfile-dependencies");
        let path = dir.join("nikaia.lock");

        let mut lock = Lockfile::new("rustc-x");
        lock.units.insert("src/main.nika".into(), record("fn main"));
        lock.dependencies
            .insert("regex".into(), BTreeSet::from(["1.13.1".to_string()]));
        lock.save(&path).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("regex") && text.contains("1.13.1"),
            "a reviewer and a second machine can see what was linked:\n{text}"
        );
        assert!(
            !text.contains(&dir.display().to_string()),
            "and nothing absolute went in with it (ADR-005 D8):\n{text}"
        );

        let reloaded = Lockfile::load(&path, "rustc-x").unwrap();
        assert_eq!(reloaded.dependencies, lock.dependencies);

        // The key is built from the unit's own inputs and the identity, and a
        // bumped dependency is none of those.
        let before = Key::build(
            "0.1.0",
            &lock.toolchain,
            &choices(),
            "src/main.nika",
            lock.units.get("src/main.nika").unwrap(),
        );
        lock.dependencies
            .insert("regex".into(), BTreeSet::from(["1.14.0".to_string()]));
        let after = Key::build(
            "0.1.0",
            &lock.toolchain,
            &choices(),
            "src/main.nika",
            lock.units.get("src/main.nika").unwrap(),
        );
        assert_eq!(
            before, after,
            "a dependency bump changes what Cargo compiles, not what this \
             compiler emits - keying on it would throw away every lowering for \
             nothing (ADR-021 D7)"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Byte-determinism, in the sense ADR-005 D8 requires: the same inputs
    /// discovered in a different order are the same file.
    ///
    /// The dependency record is a map, and D8's first banned item is iterating
    /// an unordered one to produce output. `BTreeMap`/`BTreeSet` is what makes
    /// that unrepresentable here; this is the test that says so, and the
    /// cross-process half is in `crates/nikaia/tests/project.rs`, where two
    /// `nikaia build` processes are compared.
    #[test]
    fn the_dependency_record_does_not_depend_on_discovery_order() {
        let names = ["regex", "aho-corasick", "memchr", "windows-sys"];

        let mut forwards = Lockfile::new("rustc-x");
        for name in names {
            forwards
                .dependencies
                .insert(name.into(), BTreeSet::from(["1.0.0".to_string()]));
        }
        let mut backwards = Lockfile::new("rustc-x");
        for name in names.iter().rev() {
            backwards
                .dependencies
                .insert((*name).into(), BTreeSet::from(["1.0.0".to_string()]));
        }

        let dir = scratch("lockfile-order");
        let (a, b) = (dir.join("a.lock"), dir.join("b.lock"));
        forwards.save(&a).unwrap();
        backwards.save(&b).unwrap();
        assert_eq!(
            std::fs::read(&a).unwrap(),
            std::fs::read(&b).unwrap(),
            "the order the dependencies were found in is not a fact about the build"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// The lookup rehashes the recorded assets from disk, so editing one is a
    /// miss even though the `.nika` source is untouched.
    #[test]
    fn a_changed_asset_invalidates_the_unit() {
        let dir = scratch("assets");
        let asset = dir.join("schema.sql");
        std::fs::write(&asset, "CREATE TABLE a;").unwrap();

        let mut cache =
            Cache::open(dir.join("nikaia.lock"), dir.join("store"), "t", "0.1.0").unwrap();
        let mut assets = BTreeMap::new();
        assets.insert("schema.sql".to_string(), sha256_hex(b"CREATE TABLE a;"));
        cache
            .record("a.nika", "src", assets, &choices(), &emitted())
            .unwrap();

        assert_eq!(
            cache.lookup("a.nika", "src", &choices(), &dir),
            Some(emitted())
        );

        std::fs::write(&asset, "CREATE TABLE b;").unwrap();
        assert!(cache.lookup("a.nika", "src", &choices(), &dir).is_none());

        // A recorded asset that is gone is a miss, not a panic.
        std::fs::remove_file(&asset).unwrap();
        assert!(cache.lookup("a.nika", "src", &choices(), &dir).is_none());

        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod prune_tests {
    use super::*;

    /// An entry of `size` bytes under `root/kind/`, last used `used_ago` ago.
    fn entry(root: &Path, name: &str, size: usize, used_ago: std::time::Duration) -> PathBuf {
        let dir = root.join("rlib").join(name);
        std::fs::create_dir_all(dir.join("debug")).expect("a cargo-shaped tree");
        std::fs::write(dir.join("debug/artifact"), vec![b'x'; size]).expect("something in it");
        touch(&dir);
        set_used_at(&dir, std::time::SystemTime::now() - used_ago);
        dir
    }

    /// The stamp's mtime, set directly - a test cannot wait a day.
    fn set_used_at(dir: &Path, when: std::time::SystemTime) {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(dir.join(USED))
            .expect("the stamp is there");
        file.set_modified(when).expect("set the time");
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nikaia-prune-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch root");
        dir
    }

    const DAY: std::time::Duration = std::time::Duration::from_secs(60 * 60 * 24);

    /// **Over the limit, the entry used longest ago goes first**, and only as
    /// many as the limit needs: t1 < t2 < t3, room for two.
    #[test]
    fn the_entry_used_longest_ago_goes_first() {
        let root = scratch("order");
        let t1 = entry(&root, "t1", 100, 3 * DAY);
        let t2 = entry(&root, "t2", 100, 2 * DAY);
        let t3 = entry(&root, "t3", 100, DAY);

        let pruned = prune(&root, 250);

        assert!(!t1.is_dir(), "the one used longest ago went");
        assert!(t2.is_dir() && t3.is_dir(), "the limit needed only one gone");
        assert_eq!(pruned.removed, vec![(t1, 100)]);
        assert_eq!(pruned.size, 200);
        let _ = std::fs::remove_dir_all(root);
    }

    /// **An entry in use is never removed**, however long ago its last use was
    /// stamped: the next one in the order goes instead.
    #[test]
    fn an_entry_in_use_is_skipped() {
        let root = scratch("held");
        let t1 = entry(&root, "t1", 100, 3 * DAY);
        let t2 = entry(&root, "t2", 100, 2 * DAY);
        let t3 = entry(&root, "t3", 100, DAY);
        // A build holds t1: the lock `use_entry` takes, without its stamp.
        hold(&t1);
        set_used_at(&t1, std::time::SystemTime::now() - 3 * DAY);

        prune(&root, 250);

        assert!(t1.is_dir(), "a build holds it");
        assert!(!t2.is_dir(), "the next oldest went instead");
        assert!(t3.is_dir());
        let _ = std::fs::remove_dir_all(root);
    }

    /// **Age alone removes nothing**: a month-old entry within the limit stays,
    /// and `0` is no limit at all.
    #[test]
    fn within_the_limit_nothing_goes() {
        let root = scratch("within");
        let old = entry(&root, "old", 100, 30 * DAY);
        let new = entry(&root, "new", 100, DAY);
        assert!(prune(&root, 200).removed.is_empty());
        assert!(prune(&root, 0).removed.is_empty());
        assert!(old.is_dir() && new.is_dir());
        let _ = std::fs::remove_dir_all(root);
    }

    /// What carries no stamp is not an entry, and is never removed.
    #[test]
    fn what_has_no_stamp_is_left_alone() {
        let root = scratch("unstamped");
        let loose = root.join("tools").join("x");
        std::fs::create_dir_all(&loose).expect("a directory");
        std::fs::write(loose.join("f"), vec![b'x'; 500]).expect("a file");
        assert!(prune(&root, 1).removed.is_empty());
        assert!(loose.is_dir());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn sizes_are_read_with_their_suffixes() {
        assert_eq!(parse_size("8G"), Some(8 << 30));
        assert_eq!(parse_size("500m"), Some(500 << 20));
        assert_eq!(parse_size("2GB"), Some(2 << 30));
        assert_eq!(parse_size("1024"), Some(1024));
        assert_eq!(parse_size("0"), Some(0));
        assert_eq!(parse_size("lots"), None);
        assert_eq!(parse_size(""), None);
        assert_eq!(show_size(8 << 30), "8.0 GiB");
        assert_eq!(show_size(12), "12 B");
    }

    /// **`nikaia cache clean` empties the cache**, and an absent one is not an
    /// error.
    #[test]
    fn clean_empties_the_cache() {
        let root = scratch("clean");
        entry(&root, "k", 1, DAY);
        clean_dir(&root).expect("cleaned");
        assert!(!root.exists());
        clean_dir(&root).expect("nothing to clean is fine");
    }

    /// A cache that cannot be pruned is a fuller disk, not a failed build - the
    /// rule ADR-021 D12 already gives for a cache that cannot be opened.
    #[test]
    fn pruning_what_is_not_there_is_not_an_error() {
        assert!(
            prune(Path::new("/nowhere/at/all/nikaia"), 1)
                .removed
                .is_empty()
        );
    }
}
