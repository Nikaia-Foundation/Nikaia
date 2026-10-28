---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #916, `ref Bytes?` parameters and matches over a cloned option (lowering lane; absorbs #913, #914)

Score: the bootstrap week (`docs/bootstrap-week.md`): the lowering lane's first item, in the fix checkout C3. S=3 (P3b met it; every later move over `Bytes?` or a cloned option meets it again), C~5-10 M. `Fixes #916, #913, #914`.

**Owns:** `crates/nikaia-std/src/tools/emit_core_args.nika`, `emit_core_flow.nika`, `check_facts.nika`, `check_reads.nika`, the other `emit_*` files the fix needs (not `emit_grammar.nika`), new tests under `tests/compiler/tests/`, the generated `tools/lowered/*`. **Not yours:** any `crates/nikaia/src/*.rs` but `text_tiers.rs`, `view_changes.rs`, `type_rules.rs`, `one_variant.rs`, `views.rs`; the move lanes' files.

**Note:** at 11:30 the main checkout had uncommitted edits to `check_reads.nika`, `check_core_leaves.nika`, `check_core_stmt.nika` and two untracked tests (`a_map_read_a_lambda_hands_back_is_copied_out`, `a_text_walked_through_iter_compares_with_owned_text`) of another agent. You work from `origin/develop` in C3 and do not see them; if your fix needs the same lines, stop and report under `Needs:` (the main session sorts it out).

## Issue text (#916, copied)
1. A `ref Bytes?` parameter lowers to `Option<&[u8]>`, but its argument is opened with `.as_ref()` (`Option<&Bytes>`): E0308. `emit_arg_opening_inside` (`crates/nikaia-std/src/tools/emit_core_args.nika:38`) names only `String`/`str` for `.as_deref()`.
2. `want?.clone()` of such a parameter lowers to `.to_owned()` (a `Vec<u8>`, not `Bytes`), and `expected == got` then fails (`&Vec<u8> == [u8]`).
3. A `match` over an owned option (`match t.files[0].1?.clone() { null => .., g => g != w }`) binds `Some(ref g)` (`lent_bindings`, known only after the arm body), but the comparison is not marked in `compared_views` (`local.lent` is false, since the subject is no view), so `&String != String` / `&Bytes != Bytes` fails in rustc. Also with `String`. (#914 is this defect as filed separately: check its probe is covered by step 3's tests; if not, it is `Found:`.)

Probes (expected: both compile and run; the second prints `differs`):
```
fn as_expected(want: ref Bytes?, got: ref Bytes) -> bool {
    return match want?.clone() { null => true, expected => expected == got }
}
fn main() {
    let w: Bytes? = Bytes([120])
    let g: Bytes = Bytes([120])
    println(f"{as_expected(w, g)}")
}
```
```
fn main() {
    let w: String = f"{1}"
    let t: Vec[(String, String?)] = [("a", "x")]
    match t[0].1?.clone() { null => println("none"), g => { if g != w { println("differs") } } }
}
```
Locations: `emit_arg_opening_inside` in `tools/emit_core_args.nika`; `lent_bindings` / `compared_views` in `tools/emit_core_flow.nika`, `tools/check_facts.nika`, `tools/check_reads.nika` (`nikaia run scripts/where.nika -- lent_bindings compared_views emit_arg_opening_inside`).

## To build
1. Defect 1: a `ref Bytes?` argument is opened with `.as_deref()`; test `tests/compiler/tests/bytes_option_param.nika` + `.stdout` (`true`). Commit, `Part: #916`.
2. Defect 2: `?.clone()` of a lent `Bytes?` gives an owned `Bytes`, the comparison compiles; test. Commit.
3. Defect 3: a comparison with a binding of a match over an owned option that binds `ref` is marked; tests for `String` and `Bytes`. Commit `... Fixes #916, #913, #914`.
Each test fails on `origin/develop`'s compiler before the fix (say so under `Tests:`).

## Done when
`nikaia test tests/compiler --both-settings` (every count as before plus the new tests); the guard set of `.claude/agents/builder.md`; clippy clean; lower-std fixpoint after the rebase (every lowered file the fix re-lowers is committed with it). Changelog: one line for the supervisor under `Changelog:` (a lowering fix raises the version; you do not).

## Stop and report if
A fix needs a file a move lane owns; more than one `.stderr` changes; the fix changes the lowered output of more than ten tools files in a way that is not the defect's (name them).

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; worktree lane, `docs/bootstrap-week.md` section 3)
* Read `docs/builder-facts.md` first. **You work in the worktree `/home/user/nikaia-fix`, not in `/home/user/Nikaia`** (the driver's builder works there; never edit, build or stage anything in it except your report file). Set up once: `cd /home/user/Nikaia && git fetch origin develop && (test -d /home/user/nikaia-fix || git worktree add --detach /home/user/nikaia-fix origin/develop)`; then `cd /home/user/nikaia-fix && git status` (another agent of your checkout may have left files: not yours, report them). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/nikaia-fix/target/fast/nikaia`, never another checkout's binary.
* `git add <files>` only; commit after each step. Push: `git fetch origin develop && git rebase origin/develop`, rebuild, `lower-std` once more, commit any lowered file that changed (`lowered: regenerate after rebase`), then `git push origin HEAD:develop`; rejected: again, at most three times, then `Pushed: rejected`. A conflict in `tools/lowered/*.rs` or `tools/lowered.rs` is never merged by hand: `git checkout --theirs -- <file>`, `git rebase --continue`, rebuild, `lower-std`, commit.
* Commit every lowered file `lower-std` changed, yours and others' (a compiler change re-lowers other files; that is correct in your tree). Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap-week.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most three checkouts with `target/` this week (C1, C2, C3); if `df -h /home/user` shows under 4 GB free, delete `/home/user/nikaia-fix/target` and report `blocked`. At the end delete `/home/user/nikaia-fix/target/debug` and `/home/user/nikaia-fix/target/release/nikaia-scratch` (keep `/home/user/nikaia-fix/target/fast` for the lane's next brief) and report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/lane-lowering-916-ref-bytes-option.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
