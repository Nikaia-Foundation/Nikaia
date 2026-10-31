Result: blocked
Files: none
Commit: none
Pushed: none
Tests: the move is complete on branch wip/p5c-project-drive (cc40334f, not on develop): lower-std is clean and the #1060 guard holds (only project_drive.rs and lowered.rs). With lowered/project_drive.rs:151 wrapped in Some by hand, `cargo check --profile fast -p nikaia` is clean. Build, it tests, fmt and clippy were not run because the lowered file does not compile without that hand edit. My files are restored in the shared checkout by name.
Gate: red
Deviation: none
Found: lowering defect: a value in a `T?` slot of a tuple, inside a `return match` arm, is not wrapped in Some. The checker accepts it and rustc refuses (E0308). Probe: `fn pick(x: String?) -> (i32, String?) { return match x { null => (1, null), s => (2, s.clone()), } }`, run from main, gives "Mismatched types ... Try wrapping the expression in `Some`". `return (2, "a")` lowers fine. Seen at project_drive.nika `return match built.binary { null => (101, null), binary => (host.run_directly(binary), binary.clone()) }`.
Needs: architect decision. The bug lane fixes the defect; then rerun from wip/p5c-project-drive: lower-std, build, tests, plan rows P5c/P5e, the self_hosting.py row, the rs2nika score, commit. Design on the branch: ProjectDriveRefusal gains Failed(String) for plain-anyhow texts (toolchain, nothing_exported, fs); orchestrator toolchain_can_build_for returns bool and its text moves to Nikaia (project_drive_no_std_for); added host getters shown_as, allowlist, root_tag, package_name, bind; Rust dependencies_first removed (dead). Process note: another session's check_references/emit_core_args .nika and lowered files appeared modified during this run (not mine).
Changelog: none
Owner: none
Disk freed: 0 GB; target/nikaia-project-tests 4.2G, / 8.2G free; no new cache entry over 100 MB
Rs2nika: not scored (blocked before the commit; score the draft against wip/p5c-project-drive's project_drive.nika on the rerun)
