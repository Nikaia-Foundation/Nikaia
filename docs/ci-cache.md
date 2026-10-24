# CI: what a run spends, and what a cache can save

The owner's question (2026-10-24): a CI run takes about 30 minutes; as the
compiler becomes modular (one Rust unit per `.nika`, #667 step 4,
[`modular-compiler.md`](modular-compiler.md)), can CI rebuild only what
changed? Companion to [`long-jobs.md`](long-jobs.md). Durations below are
GitHub's step timestamps, given as shares; #795 holds the runs.

## Where a run goes (three develop runs, 2026-10-24)

* **Up to 40 % is waiting, not working.** The workflow's `concurrency` group
  queues `develop` runs: a push waits until the run before it ends (one run
  waited 40 % of its length, another 14 %), and a third push while one waits
  cancels the waiting one (7 of the last 12 runs were cancelled that way).
* **Two jobs run in parallel and decide the length**: `bootstrap_compiler` and
  `arm64`; `changes` is ahead of both, `promote` behind. `arm64` was the longer
  one in two runs of three.
* **Inside each job, `Test` is 68-83 %.** `bootstrap_compiler`: Test 68-78 %,
  accepted-programs sweep 12 %, `Build the workspace` 10-11 %, cache restore
  2-6 %, clippy 3 %, the Python checks under 1 %. `arm64`: Test 78-83 %, build
  11-14 %, cache restore 6 %.
* **`Test` mixes two things nobody has separated**: compiling and linking the
  267 integration-test binaries at `release` (each links the compiler and
  `std`), and running them, where 125 of the 267 files hand emitted programs to
  `rustc`. `cargo test` also runs the binaries one after another, so a binary
  with a few slow tests leaves the other cores idle.

## The caches there are, and whether they are sound

* **`actions/cache` on `target` (today).** Key: OS (and arch on arm64),
  `Cargo.lock`, `rust-toolchain.toml`. The lock file last changed on
  2026-10-21, so every run is an exact hit, and `actions/cache` never saves on
  an exact hit: each run restores the same three-day-old `target`. A fresh
  checkout gives every source file a new mtime, so Cargo rebuilds every
  workspace crate anyway; only the third-party crates are reused. Sound, but it
  carries old workspace artefacts and test binaries for nothing.
* **A dependencies-only cache** (`Swatinem/rust-cache`): keyed on the Rust
  version, the lock file, the job and the arch; it deletes the workspace's own
  artefacts before saving. Same gain as today, smaller restore, refreshed when
  a key changes. Sound: third-party crates are pinned by version.
* **Incremental compilation across runs.** It is off in CI because an
  incremental `target` is several GB, a save and restore per run can cost more
  than it saves, and with one large unit (today's 7.4 MB `tools` unit) nearly
  every commit re-optimises most of it anyway. After the module split it can
  pay: build with the `fast` profile (incremental, 256 units) and restore the
  previous `develop` run's `target/fast` (key ending in the run id,
  `restore-keys` the prefix). The mtime problem does not bite: Cargo re-runs
  `rustc`, and `rustc` reuses unchanged units by content hash, not by mtime.
* **sccache** caches whole `rustc` invocations by content hash; it does not
  cache binaries or incremental crates. Our two large crates change in almost
  every commit, so it saves what the dependencies cache saves: not worth it.
* **The `lower-std` per-file cache** (`~/.cache/nikaia/lower-std`) is keyed on
  the compiler binary's own hash, which every Rust change moves, and the tests
  that lower run inside a test binary whose hash moves too: restored in CI it
  would always miss. Not cached; revisit if its key becomes the lowering's
  sources rather than the binary.
* **The compiled-`std` cache** (`~/.cache/nikaia/rlib`, a Cargo target dir
  keyed on toolchain, target, codegen, features, CPU): Cargo's own fingerprints
  decide freshness inside, so it is sound to restore and rebuilds `std` after
  a checkout like any path crate. Gain unmeasured.
* **Test binaries**: not cached; they depend on the compiler, which every
  commit changes. One integration-test binary (#793 option B) turns 267 links
  into one. **cargo-nextest** runs all binaries' tests in one pool across the
  cores, and `--partition count:k/n` with `nextest archive` splits the run
  over k machines that share one build.
* **x86 and arm64** need separate entries (different object code); the keys
  name `runner.arch`. GitHub keeps 10 GB per repository and evicts the least
  recently used entries, and any entry unused for 7 days: two arches with an
  incremental `target` each are near that limit, which argues for caching the
  incremental build on x86 only.

## What keeps a cache from turning red into green

`promote` moves `main` on a green run, so a stale artefact taken for fresh is
the one failure that matters.

1. Restore only what is checked by content or by Cargo's fingerprints: the
   dependencies cache, `rustc`'s incremental store, the compiled-`std` tree.
2. **Never restore source mtimes** (`git restore-mtime` and the like): with
   them Cargo would take a crate built from an older tree as fresh whenever
   the changing commit is older than the cache. Nightly's content-hash
   freshness (`-Z checksum-freshness`) is not ours (ADR-001 D1, stable only).
3. Keys name the lock file, `rust-toolchain.toml`, OS, arch and profile.
4. Only `develop` pushes save; pull requests and schedule runs read.
5. One clean build per push stays: if x86 builds incrementally, `arm64` stays
   a plain `release` build of the same commit, and the weekly run uses no
   build cache at all.

## Order

Now, independent of the module split: separate building the tests from
running them (to see the shares), nextest's pool, the dependencies-only
cache, then shards if the test run still dominates. After #667 step 4: the
incremental x86 build, judged by units re-optimised per commit. Not a cache
but the largest single share: running `develop` runs side by side instead of
queueing them (owner's choice; `promote` already moves `main` only forward).
Build order and tests: #795.
