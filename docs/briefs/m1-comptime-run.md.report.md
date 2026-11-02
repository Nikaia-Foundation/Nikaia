Result: partial
Files: crates/nikaia-std/src/tools/comptime_tree.nika crates/nikaia-std/src/tools/lowered/comptime_tree.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/comptime_run.rs
Commit: comptime_run: the readers' fixpoint, the value function's names and the declared comptimes move into comptime_tree.nika
Body: Moved from comptime_run.rs (about 45 Rust lines): readers_dropped's loop (now comptime_readers_dropped, the host gives one unit with drop_readers), ComputeHost::value_fn_names's walk (comptime_value_fn_names) and comptimes_of's filter_map (comptime_declared_of). without_readers keeps its signature, so grammar_run.rs does not change. Output is the same. Self-hosting share 84.9 % -> 84.9 % (Nikaia lines 101691 -> 101726, HEAD vs index). Left in Rust: process spawning, threads, statics/Mutex, pid names, the Lowered/emit dispatch in driven (moving it adds Rust), artifacts_in (JSON).
Part: #125
Tests: cargo test --lib comptime ok (0 match); it -- comptime self_hosting:: 52 passed; tests/compiler *comptime* 12/12 ok; lower-std fixpoint holds (comptime_tree.rs byte-identical after the second lower-std)
Gate: red badges (self-hosted.svg out of date, supervisor's); clippy -D red only on another agent's lowered/threads.rs dead code, comptime_run.rs lint-free
Deviation: ComptimeRan was not reshaped to carry CheckWall (it would drop the 6-arm Wall copy in run_comptime): blocked by the defect under Found. artifacts_in not moved: tools have no JSON reading in std.
Found: lowering defect: an arm-bound name matched by an inner `match` whose arm takes a non-copy part is lowered as `Ran::Hit(ref wall) => match *wall { Wall::Refused { detail } => detail ...}` (E0507). Probe: enum Wall { Refused { detail: String }, Nowhere }; enum Ran { Answer(String), Hit(Wall) }; fn told(r: Ran) -> String { return match r { Ran::Answer(t) => t, Ran::Hit(wall) => match wall { Wall::Refused { detail } => detail, Wall::Nowhere => "n" } } }. Writing `let w = wall; match w` lowers correctly. Suspect check_compare.nika parts_lent / check_core_flow_lent_parts (the inner match's taking does not count as giving `wall` away). Not fixed: did not fit the time box.
Needs: bug-lane fix of the defect (then ComptimeRan can become Answer | Wall(CheckWall)); supervisor decision on JSON reading for tools (artifacts_in)
Changelog: none (internal move)
Owner: none
Disk freed: none; target/nikaia-project-tests 1.3G, / 4.9G free; new entries over 100 MB: rlib/df8bcfe6 269 MB, run-bundle/3f8b93b8 342 MB
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
