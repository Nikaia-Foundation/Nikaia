---
agent: builder
model: sonnet
max_turns: 110
budget_usd: 20
---
# Brief: #867 part 2, the `Seeds` tables and their type queries move to `tools/emit_seeds.nika`

Score: self-hosting lane (`docs/priorities.md`, E11-4b part 2): about 130 Rust lines of `emit/seeds.rs` (623 left after part 1, 38697bf). One step, so `done` is reachable; the reader walks (`read_with`, `read_barriers`, `hands`, `self_handers`, `count_types`, lines 94-495) are the next brief, not this one.

## Facts
Part 1 moved the writer: `EmitSeedsPlace`, `emit_seeds_write` in `crates/nikaia-std/src/tools/emit_seeds.nika`; `Seeds::write` (seeds.rs:617) passes `arity` as a callback. What stays Rust in `crates/nikaia/src/emit/seeds.rs`:
* `struct Seeds` (28-57): `types: BTreeMap<String, usize>`, `calls: BTreeMap<String,(usize,usize)>`, `functions: bool`, `barriers`, `fixed`, `private`, `excluded` (`BTreeSet<String>`), `trait_maps: bool`, `shapes: BTreeMap<String, Shape>` (`contracts::maps::Shape`, Rust).
* Free leaves: `root_name` (60, walks `ast::Expr`), `bare` (70), `last` (75), `is_map` (79).
* Queries (497-613): `is_fixed`, `barrier_names`, `trait_maps`, `count`, `distinct`, `key`, `mentions_a_struct`, `positions`, `shape`, `is_empty`, `calls`, `arity`. They read `parsed.text(ty.name)` and `ty.code/is_tuple/is_view/is_nullable/is_slice/generics`.

## To build
1. A Nikaia struct `EmitSeedsTables` in `emit_seeds.nika` holds every field of `Seeds` except `shapes` (stays Rust until the reader moves: `Shape` is the ledger's). Rust `Seeds` holds it as one field (`tables`), the reader fills it through it. The leaves and the queries above, except `shape`, become `impl EmitSeedsTables` methods / `emit_seeds_*` functions in Nikaia; a `ref Type` with a `ParsedView` (`tools/parsed_view.nika`, pattern `check_items.nika:745 item_held_inline`) or `words: ref winnow_grammar::InternerContext` for the name. `Seeds`' public methods stay as one-line adapters so the 13 callers outside `seeds.rs` (`emit/host.rs`, `emit/mod.rs`, `contracts/maps.rs`) are unchanged. `emit_seeds_write` takes `ref EmitSeedsTables` instead of the `arity` callback if that is now simpler.
2. Output of `lower-std` unchanged outside `emit_seeds.rs`/`lowered.rs`. Update the `scripts/self_hosting.py` row and the *Seeds* line of `docs/selfhosting-plan.md` (what is left: the reader walks). Commit `Part of #867`.
rs2nika draft first (`crates/rs2nika/convert`, `--profile emit --prefix emit_seeds_`), report `rs2nika: kept N of M` in the commit message. A new method on an existing `.nika` needs no extra `lower-std` before `cargo build`.
Tests: none new unless a defect is met (then a `tests/compiler/` test, own commit). Run `cargo test --profile fast -p nikaia --test it -- seeds maps sysroot`, `nikaia test --both-settings --project tests/compiler`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b867-seeds-tables.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
