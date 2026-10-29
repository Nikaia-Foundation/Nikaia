Result: done
Files: crates/nikaia/src/project.rs crates/nikaia-std/src/tools/project_settings.nika crates/nikaia-std/src/tools/lowered/project_settings.rs crates/nikaia-std/src/tools/lowered.rs assets/badges/self-hosted.svg docs/selfhosting-plan.md
Commit: project: program name, root tag, own copy and trace into Nikaia (P5a)
Body: Committed by hand, not with commit.nika: lowered.rs also held another session's uncommitted text_tiers index hunk, so only the project_settings hunk was staged (git apply --cached). 63db333 is the move, db0b1f0 the plan row. root_tag uses hex.scalars.take(16) because NK1214 refuses hex[0..<16]. trace takes TRACE_VAR's name because drive_to :2038 still reads it. own_copy's refusal goes to Rust through `refusal` (plain anyhow, like the old with_context) rather than `refused`. The old error chained the io cause under "copying A to B"; the new one does not. A failed create_dir now prints std's IoError text.
Fixes: none
Pushed: 63db333 db0b1f0 pushed
Tests: it `project cpu shared_target own_copy trace program` plus the guard set: 202 passed, 3 failed in the shared run (map_seeds and accepted_compiles: libnikaia_std rlib deleted by another session's rebuild; project::a_program_another_compiler_lowered_is_lowered_again: compiler identity changed mid-run, after its two trace asserts had passed). All 3 passed when rerun alone. `--lib project::` 10 passed. lower-std fixpoint holds. Self-hosting 82.2 % (82.1 % before; measured on HEAD plus only this change).
Gate: green
Deviation: none
Found: none
Needs: supervisor: clippy -D warnings is red in the working tree on another session's uncommitted crates/nikaia-std/src/tools/lowered/view_changes.rs:722 (clippy::map_identity, from view_changes.nika). My files pass clippy with only that lint allowed.
Changelog: none
Owner: none
Disk freed: 0 GB; target/nikaia-project-tests 2.2G, / 3.7G free; new entries over 100 MB: run-bundle/194302eb3b5f1d47 285 MB, build-time-parsers/768d2772d43bb4cc 474 MB
Rs2nika: kept 7 of 27 lines (under 60 %); costliest edits: impl methods redone as project_settings_ free functions over root/variant (the brief's Shape); orchestrator::cache::sha256_hex(named.bytes)[..<16] became host.sha256_hex plus scalars.take(16) (NK1214); trace's OpenOptions/push_str and Invocation parameter became fs::write(; append: true) with crate_name/is_probe parameters
