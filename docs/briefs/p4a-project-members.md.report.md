Result: done
Files: crates/nikaia-std/src/tools/project_members.nika crates/nikaia-std/src/tools/lowered/project_members.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/project.rs crates/nikaia/src/modules.rs scripts/self_hosting.py docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: project: a package's members into Nikaia (P4a)
Body: packages_of, renames_in, dependencies_first, members_of, canonical and crate_root_of move to tools/project_members.nika; rooted (toml::Value) stays Rust. Manifest and toml stay Rust: the walk reads a manifest as ProjectMembersManifest (name, keys, path dependencies, foreign crates, has_build), through the trait ProjectMembersHost::manifest_at, which project.rs implements (Manifests: keeps each Manifest read and the error of one that did not read). Member stays Rust (it holds the Manifest for drive_to's codegen_for) and is built from ProjectMembersMember; modules::Dependency is now ProjectMembersDependency. ProjectMembersRefusal is turned back into the same anyhow error and refused! text in project.rs, so no message changes. The draft's `.chain(foreign_crates)` is a loop: `Seq` has no `chain` in std (NK1171). The lowering defects met were fixed first in 7c29014 and c472f92.
Part: #125
Tests: lib project:: 10 passed; nikaia test --both-settings --project tests/compiler 223 passed; it filtered (package dependenc member manifest workspace path_dep rename + guard set) 130 passed, 1 failed in that run (nikaia_test::the_compiler_test_package_passes_at_both_settings, drive_kept "settings disagree", running beside clippy), rerun alone it passed (1 passed), a flake of the parallel run; lower-std fixpoint (only project_members.rs new); clippy clean; self-hosting 80.7 %
Gate: green
Deviation: none
Found: std Seq has no chain (NK1171): the draft's keys.chain(foreign) became a loop
Needs: none
Changelog: none
Owner: none
Disk freed: 0 GB
Rs2nika: kept 33 of 116 lines; costliest: Manifest calls (dependencies() matched on Dependency::Path, Manifest::read) rewritten as the view's paths and host.manifest_at; refuse!/{} macros as throw ProjectMembersRefusal::Refused(f"..."); while-true + first()?/break queues as while !is_empty() { remove(0) }
Pushed: 3620d270790c76d2a31bf029be87e724c79fcb21
