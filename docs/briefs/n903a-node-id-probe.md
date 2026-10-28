---
agent: builder
model: opus
max_turns: 70
budget_usd: 15
---
# Brief: probe the id readers, and `NodeId` carries a span until numbered (#903, ADR-383)

Score: self-hosting lane (`docs/priorities.md`): the grammar rows of the `parser/mod.rs` slice plan (`docs/selfhosting-plan.md`, *Large files*) wait on #903 and #557; the owner decided the question on 2026-10-28 (ADR-383).

## Facts
* Read issue #903 (brief, gate, traps, *To build*) and `docs/specification/adr/adr-383.md` first; this brief builds the steps named below of its *To build* list, as written there.
* Do not touch the rest of `Trivia` (`semi`, `ran`, `comment`, `block_comment`, `PrTrivia::cut`): that is #557's.
* Gate: `tests/` and `tests/errors/` output byte-identical (spans reach every message): run the tests the change touches and the error tests whose spans come from expressions, not the full suite.

## To build
1. Step 1 of #903 (probe): every reader of an id beyond equality and `FIRST_BUILT`; one that depends on the old order is fixed first, its own commit. If none, say so under `Tests:` (what was probed).
2. Step 2 of #903: `NodeId::parsed()` and setting the span once (innermost first); D3 equality and `Debug` unchanged. Report `size_of::<Expr>()` before and after in `Body:`. Commit `node_id: a parsed id carries its span until numbered (#903)`; no `Fixes`.

## Stop and report if
A reader depends on the old id order in a way that changes output and cannot be fixed in one commit; `size_of::<Expr>()` grows by more than 8 bytes (report before/after, `Owner:` asks whether to go on); an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/n903a-node-id-probe.md.report.md`)
Fields as in `p1-large-files-plan.md` (`Result:` bare word; `Owner:` a question or `none`; `Deviation:`/`Found:` `none` bare). `Rs2nika: none` unless a step moves Rust. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
