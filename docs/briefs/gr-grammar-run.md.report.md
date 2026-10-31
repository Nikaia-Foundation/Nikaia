Result: done
Files: crates/nikaia-std/src/tools/grammar_run.nika crates/nikaia-std/src/tools/lowered/grammar_run.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/grammar_run.rs scripts/self_hosting.py docs/selfhosting-plan.md
Commit: grammar run: the parser's build and run move into Nikaia (GR)
Body: `compile_and_run`, `cargo` and `write_if_changed` move to `tools/grammar_run.nika` as `grammar_run_compile_and_run` and `grammar_run_write_if_changed`, over `process::run`, `hash::sha256_hex`, `env::var` and `fs` with `fs::Root::Anywhere`. The host trait `GrammarRunHost` has `driver`, `manifest`, `shared_target` and `where_tools_start`. A host method that throws lowers to `Box<dyn Error>`, so the Rust host keeps the driver's own `Wall` in a `RefCell` and `Workshop::run` returns it after the call; the Nikaia side rethrows `NowhereToBuild` there, and that value is never seen. `Workshop::run` keeps its `Mutex` guard and calls the lowered function through `block_on`, which works inside a `comptime` evaluation (the `it` grammar tests ran real builds). The file gets an `impl Error for CheckWall` (needed to throw it), which also gives `Wall` a `Display`. The unchanged-file check reuses `sysroot_holds` (sysroot_cache.nika), because `String?` == `String` is refused (NK1102). Messages stay byte for byte, but `{error}` is now an `io::IoError`'s text and not Rust's `io::Error`'s. `process::run` throws on output that is not UTF-8, where Rust read it lossily. Stays Rust: `Ask`, `Workshop` and its `Mutex`/`OnceLock`, `run_comptime`, `driver`, `dumper`, `shapes`, `decode`, and `shared_target` and `manifest` as host methods (they move with O1).
Part: #125
Pushed: local b070adb974fbfab6ded44c4709efd92e43752b7d
Tests: it `grammar_at_build_time dsl_grammar build_time comptime` 115 passed; `--lib grammar_run::` 0 tests; clippy, cargo fmt, pre-push-check green; lower-std fixpoint: only grammar_run.rs and its lowered.rs index lines are mine. Not run: tests/compiler as a whole (left to the driver's suite). No existing test prints the texts `running cargo: …`, `creating …`, `writing …`, `running <binary>: …`, a failed cargo's stderr or `DidNotBuild` (no failing cargo can be staged offline).
Gate: green
Deviation: none
Found: none
Needs: lower-std with the freshly built binary rewrites `lowered/project_members.rs` (TrustedMap -> HashMap) and `lowered/check_core_ops.rs` (another agent's .nika is modified). Neither is mine; both left uncommitted.
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: kept 9 of 51 lines; costliest edits: the `Command::new(..).arg(..)...output()` chains rewritten as `process::run(program, [args]; dir:)`; `.map_err(fn(error) {..})` turned into `catch { throw CheckWall::.. }` with `display()` in f-strings; the `impl Workshop` method over `parsed`/`ask` turned into a free function over `host.driver()`/`host.manifest()`/`host.shared_target()`. The draft also held the unmoved `shared_target`.
