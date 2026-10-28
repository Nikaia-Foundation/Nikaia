---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: the numbering pass, and the grammar draws no ids from `Trivia` (#903, ADR-383)

Score: self-hosting lane (`docs/priorities.md`): the grammar rows of the `parser/mod.rs` slice plan (`docs/selfhosting-plan.md`, *Large files*) wait on #903 and #557; the owner decided the question on 2026-10-28 (ADR-383).

## Facts
* Issue #903 text, copied (builders have no GitHub): ADR-383 (ADR-340 D1 changed in place, owner 2026-10-28): the grammar builds nodes with a parsed, unnumbered id; a numbering pass walks the finished tree in input order (node before children, children left to right) and gives every parsed node the next number from `first`, dense, the same on any number of cores. It runs in `parse_to_ast` right after the grammar and before `one_variant::expand` (ADR-347), and in `parse_expression_from` for a hole under the lock `Parsed::hole` holds. A node carries its span (innermost rule first, as `at` keeps it) until it is numbered; the pass writes `Parsed::expr_spans` as a list indexed by `id - first`. Part I-III say nothing about ids: no specification text, no version raise. Read `docs/specification/adr/adr-383.md`.
* #903 steps 1-2 are done (39ebd83, n903a): no reader depends on the old id order; `NodeId` is number + span (two u32, start `u32::MAX` where none; `node_id.rs` is in every program's std, so no `ast::Span` there). API: `NodeId::parsed()`, `at(start, end)` (keeps the first span set, innermost first), `span() -> Option<(u32, u32)>`, `is_parsed()`; `numbered`/`fresh` carry no span; `ast::spanned(expr, span)` (`crates/nikaia/src/ast/mod.rs:43`) puts it on a node. `size_of::<Expr>()` is now 88 (was 80), `Stmt` 208.
* The only reader that saw the old order is the Rust test `expr_ids::ids_follow_parse_order` (`crates/nikaia/tests/it/expr_ids.rs`): update its expectation to the new order (changing an existing Rust test to keep it passing is allowed; no new Rust test).
* Do not touch the rest of `Trivia` (`semi`, `ran`, `comment`, `block_comment`, `PrTrivia::cut`): that is #557's.
* Gate: build, clippy, `scripts/pre-push-check.sh`; `tests/` and `tests/errors/` output byte-identical (spans reach every message): run the tests the change touches, the `it` binary's `expr_ids::`, `parse_error::`, `samples::`, `diagnostics::`, `errors_lowering::`, and `nikaia test --both-settings --project tests/compiler`; not the full suite. `lower-std` must stay unchanged except the new `node_numbers` lowered file and its `lowered.rs` entry.

## Locations (`crates/nikaia/src/parser/mod.rs` unless named)
* `fold_binary` :30 and `fold_postfix` :40 call `user.reserve(..)` and pass `first` to `nikaia_std::tools::tree_build::{fold_binary, fold_postfix}` (`crates/nikaia-std/src/tools/tree_build.nika`); `guarded` :45 passes `user.id()` to `tree_build::guarded`.
* `struct Trivia` :66 (`next_id`, `spans`, `spans_from`, `id()`, `reserve()` :128, `at()`); 71 `_state.user().id()` and 27 `_state.user().at(e, _span)` in the grammar actions.
* `parse_expression_from` :649 (sets `next_id`/`spans_from` from `first`, returns the next free number); `parse_to_ast` :794, `one_variant::expand` called at :871.
* `crates/nikaia-std/src/node_id.rs` (`parsed`, `at`, `span`, `is_parsed` :65-103).

## To build
1. #903 steps 3 and 4, one commit or two: `tools/node_numbers.nika` (tree by value, ADR-094 D7, and `first`; returns the numbered tree, the span list indexed by `id - first`, the next free number), called in `parse_to_ast` before `one_variant::expand` and in `parse_expression_from`; then the 71 `_state.user().id()` become `NodeId::parsed()`, the 27 `.at()` a pure function of node and span, the folds lose `first`, `guarded` its id, `Trivia` loses `next_id`, `spans`, `spans_from`, `id()`, `reserve()`, `at()`. The last commit says `Fixes #903` and sets *Built: yes* on ADR-383's index row.

## Stop and report if
A reader depends on the old id order in a way that changes output and cannot be fixed in one commit; `size_of::<Expr>()` grows by more than 8 bytes (report before/after, `Owner:` asks whether to go on); an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/n903b-node-numbers-pass.md.report.md`)
Fields as in `p1-large-files-plan.md` (`Result:` bare word; `Owner:` a question or `none`; `Deviation:`/`Found:` `none` bare). `Rs2nika: none` unless a step moves Rust. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
