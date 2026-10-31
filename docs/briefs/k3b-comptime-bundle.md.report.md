Result: blocked
Files: none
Commit: none (the move is kept on local branch wip/k3b-comptime-bundle 9a1678a8, not built, and in /tmp/claude-0/-home-user-Nikaia/b41a9812-1f7c-495b-a8c2-0c229396ae6f/scratchpad/k3b/comptime_bundle.blocked.nika; the shared tree is restored)
Body: tools/comptime_bundle.nika lowers in full. With the two defects below patched by hand in lowered/comptime_bundle.rs, `cargo check -p nikaia-std --features compiler` is clean. The Rust adapter in comptime_run.rs on the wip branch is partial (bundle/run_bundle/run forwards plus the BundleHost impl; write_if_changed/rustc/toolchain/target_libdir are not forwards yet). Brief error: runtime_dependencies returns Vec[(String, String)], not what the brief lists.
Part: #125
Pushed: none
Tests: probe p1 (record parse with a loop) runs. Probe p2 (below) is refused by rustc. lower-std ran clean before and after. No it/compiler tests, since nothing was committed.
Gate: not run
Deviation: none committed. The `_workshop` parameter of bundle() stays only on the Rust forward.
Found: (1) A compiler defect. A tools/*.nika call to `Duration.in_nanos()` lowers to `.in_nanos()` without `use nikaia_std::time::DurationIn as _`, so rustc gives E0599. The tools preamble in sysroot.rs (~765-780, next to ByteCopy/ByteReads) lacks the prelude's time traits (DurationExt, DurationIn, InstantSince). Probe: any tools fn with `fs::metadata(p).modified().in_nanos()`. A plain `nikaia run` works. (2) A compiler defect in the #901 family. split_once parts bound by a closure's tuple pattern lower as &str. Probe: `lines.iter().skip(1).filter_map(fn(line) { line.split_once("=") }).map(fn((name, rlib)) { (name.clone(), as_path(rlib)) }).collect()` into Vec[(String, fs::Path)], with `as_path(text: String) -> fs::Path`. rustc: expected Vec<(String, PathBuf)>, found Vec<(&str, PathBuf)>. `.clone()` and a typed let fail the same way. A `for (name, rlib)` loop works, but the chain stays a chain. (3) After my lower-std, lowered/project_members.rs came out with HashMap -> TrustedMap (7 lines) and no .nika change of mine (#1060-like). I restored it by name.
Needs: bug lane fix of (1) the tools preamble in sysroot.rs and (2) split_once parts in a closure tuple pattern, each with a tests/compiler program. Then re-run K3b from wip/k3b-comptime-bundle.
Changelog: none
Owner: none
Disk freed: none; target/nikaia-project-tests 4.2G, / 8.9G free; new cache entry over 100 MB: run-bundle/3f8b93b88f7f213d 425 MB
Rs2nika: none (nothing committed; score at the re-run)
