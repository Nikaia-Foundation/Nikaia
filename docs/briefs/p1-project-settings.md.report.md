Result: done
Files: crates/nikaia-std/src/tools/project_settings.nika crates/nikaia-std/src/tools/lowered/project_settings.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/project.rs crates/nikaia/src/main.rs crates/nikaia/tests/it/reentrancy_switch.rs docs/selfhosting-plan.md scripts/self_hosting.py assets/badges/self-hosted.svg
Commit: project: the build settings' words into Nikaia (P1)
Body: `refuted_claims`, `unproved_claims`, `optimizations`, `prove_through_loops`, `skip_free_at_exit`, `renamed` (as `project_settings_renamed`), `artifact`, `cpu_config`, `choices`' key (`choices_key`), `panic_strategy` and `encoded_with_cpu` move to `tools/project_settings.nika`; `symbol_prefix` stays Rust (#924). `Settings` is now `{ build: Build, words: ProjectWords }`, the strings a Nikaia struct; field reads became `settings.words.x` (project.rs, main.rs, reentrancy_switch.rs). The refusals are a thrown `ProjectSettingsRefusal::Refused`, turned back into the same `anyhow!` text by `refusal` in project.rs, every message byte for byte. `Settings::cpu_config`, `choices`, `panic_strategy` stay as one-line forwards (callers unchanged); `choices` computes `cpu_key` in Rust only where a level is named and passes it in, with the `Build`'s bounds, overflow and two optimization flags. `optimizations` stays a pub Rust forward (the `it` tests use it), `prove_through_loops`/`skip_free_at_exit` are re-exported. `cpu_comes_after_the_users_rustflags` stays a Rust unit test calling the lowered `encoded_with_cpu` (a tools function is not reachable from a `tests/compiler` program). No use of #909 or #916.
Fixes: none
Part: #125
Pushed: aab4d7492cba172dd4eb9923da058fefb1cf8f20
Tests: lib project:: 10 ok; it project:: c_library bounds_checks target_has simd assert + guard set 131 ok; reentrancy_switch:: skip_free_at_exit_is_on_or_off the_overflow_option_is_read 7 ok; nikaia test --project tests/compiler 226 ok; lower-std fixpoint; clippy clean; self-hosting 80.9 %
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: kept 29 of 86 lines; costliest: (1) `with` rewritten `with_` inside string literals (three refusal messages); (2) `anyhow!` kept instead of a thrown module refusal, and `let ... else` / `.is_some_and` / `matches!` left untranslated; (3) `choices`' 12-hole `format!` broken (TODO) and the `&self` methods drafted as `state: ref CheckState ... sync` despite `--profile check-free`
