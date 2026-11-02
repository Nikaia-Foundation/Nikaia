Result: done
Files: crates/nikaia-std/src/tools/sysroot_package.nika crates/nikaia-std/src/tools/lowered/sysroot_package.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/lib.rs crates/nikaia/src/sysroot.rs scripts/self_hosting.py docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: sysroot: the tools package is assembled in sysroot_package.nika (S1)
Part: #125
Body: HAND_WRITTEN, Codegen::render, every_refusal (generic host trait SysrootPackageHost, Rust answers with diagnostics::render_finding), without_bodies, shared, raw_string, facade and the assembly of lower_tools_files_cached moved into sysroot_package.nika (byte scan with `text.bytes[a..<b].text()`); sysroot.rs keeps one-line forwards, Codegen::new, the parallel lowering, the cache, Phases. HAND_WRITTEN is a `comptime ref Array[SysrootPackageHand]` (a Vec or tuples is NK1167). `clippy::needless_as_bytes` joins the tools allow list (`text.bytes.len()`). Plan row S1 marked built.
Pushed: local 7189b7fc7ce93129782b9c0996df493456a1ad4a
Tests: lower-std fixpoint with the new binary leaves every lowered/*.rs and lowered.rs unchanged (lowered.rs only gains the new module's lines); cargo test --lib sysroot:: 8 ok; it sysroot:: selfhosting_defects:: self_hosting:: 36 ok; clippy -D warnings clean
Gate: green
Deviation: none
Found: keeps.nika (K4's uncommitted work) broke lower-std and cargo check of the shared tree for a while; I lowered my file in a scratch NIKAIA_SYSROOT copy with HEAD's keeps.nika, and built once the tree compiled again
Needs: none
Changelog: The toolchain's package assembly (refusals, interface without bodies, pub(crate), facades, index) is written in Nikaia (sysroot_package.nika)
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
