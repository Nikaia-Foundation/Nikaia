Result: partial
Files: crates/nikaia-std/src/tools/describe.nika crates/nikaia-std/src/tools/lowered/describe.rs crates/nikaia/src/describe.rs crates/nikaia/src/manifest.rs assets/badges/self-hosted.svg
Commit: 9363837d describe: adopt's `# crate:` header parse; 4f447d49 describe: the unanswered-names filter; 020e1f0b describe: rust_dependency (describe.rs :166-170, :182-186, :261-292 into describe.nika)
Body: Self-hosting share 86.7 % (104279) before, 86.7 % (104309) after (shared tree); badge from the committed tree 86.5 to 86.6 % (committed in 020e1f0b). describe_adopt_refusal now takes the shipped text and reads the header itself (describe_header_version); describe_unanswered keeps order and the `crate::name` form; describe_rust_dependency reads the Nikaia ManifestFile through a new accessor Manifest::file and throws Refused::Because with the same three texts, the Rust forward keeps its signature and clones the toml::Value of the answered key. Left in Rust: describe/adopt (anyhow contexts, Ledger::read_beside, render_description/render_derived of contracts/mod.rs, fs writes and copy), draft (rust::file error into anyhow, Ledger::empty), cargo_version (toml crate), the DescribeHost impl (parser); crate_sources and crate_root wait on without_dots (Found).
Fixes: none
Pushed: local 020e1f0b
Tests: cargo test --profile fast -p nikaia --test it -- describ shipped digest adopt: 74 passed after step 1; with manifest filter added after steps 2-3: 85 passed 0 failed; cargo test --lib -- manifest:: describe:: 3 passed; lower-std fixpoint clean (private copy); clippy --all-targets -D warnings clean; rustfmt clean
Gate: green
Deviation: the left glue was not all moved: without_dots, crate_sources and crate_root wait on the gap below; the writes stay Rust (they write contracts/mod.rs's Rust renderings with anyhow path contexts)
Found: std gap: fs::Path has no `components()` (nor a lexical `.`/`..` normalisation; Part III has only the filesystem `canonical`), so `without_dots` cannot move; probe `use std::fs` / `fn walk(p: ref fs::Path) { for part in p.components() { println(part.text_of()) } }` gives NK1171 "`fs::Path` has no method called `components`". pre-push `version` is red only on the shared tree's self-hosted.svg (other agents' files), the badge from the committed tree is current
Needs: supervisor decision (fs::Path components or a lexical normalise, then without_dots, crate_sources and crate_root move)
Changelog: none
Owner: none
Disk freed: 1.1 GB (private copy and scratch binaries); target/nikaia-project-tests absent; df 4.0G free
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
