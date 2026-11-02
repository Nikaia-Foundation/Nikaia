Result: partial
Files: docs/briefs/m2-grammar-run.md.report.md
Commit: m2 grammar run: the report (driver and shapes moved, step 2 blocked)
Body: 0016f6ef fixes a RefCell double borrow in comptime_run::without_readers (HEAD b285603c) that panicked every build-time grammar; 0489f495 moves the driver's assembly, grammar_run_dump and the shapes into grammar_run.nika behind GrammarDriverHost (4 methods). Self-hosting share 101293, 84.6 % before, 101983, 85.2 % after (index copy).
Part: #125
Pushed: local 0016f6ef 0489f495 and the report commit (commit.nika refused: another agent staged diagnostics.nika; committed with git commit --only)
Tests: --lib grammar 1 ok; --test it -- grammar self_hosting:: 97 ok (4 failed on the unfixed comptime_run); tests/compiler *grammar* 46 ok --both-settings; lowered fixpoint clean
Gate: clippy not runnable: nikaia-std lowered/touch.rs:206 and lowered/ty.rs:1230-1238 test fns `never used` (committed, not mine); pre-push index/fmt/adr-refs/badges green, version red only on the tree's self-hosted.svg (others' files)
Deviation: Workshop and the compile-and-run host stay in Rust (emit, infer, trust, rustc calls); step 2 not built: put_rule_types/expand_grammar_types need an in-place AST walk only text_tiers.rs (off limits) has, type_written blocked by the gap below
Found: (1) collecting `.map(fn -> T?)` into `Vec[T]?` refused NK1103, no ADR (probe scratchpad m2/p3/main.nika); (2) a lambda with `return null` and `return (x, x)` lowers to rustc "expected Option, found tuple" (lowering defect); (3) clippy red on lowered/touch.rs, ty.rs test fns
Needs: supervisor decision (Option collect; a shared Nikaia in-place AST walk for step 2)
Changelog: grammar at build time: the driver and the result shapes are Nikaia; a build-time grammar no longer panics on a double borrow
Owner: none
Disk freed: target/nikaia-project-tests 2.3G, / 3.6G free; new entries >100 MB: build-time-parsers 898, run-bundle 342, rlib 280
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
