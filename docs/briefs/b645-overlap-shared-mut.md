---
agent: builder
model: sonnet
max_turns: 90
budget_usd: 15
---
# Brief: #645 item 1, two `overlap` branches that change one `mut` binding are refused with `NK2104`

Score: bugs lane (`docs/priorities.md`): a program `nikaia check` accepts fails in rustc (Part III C.1). Items 2 and 3 of #645 no longer reproduce (architect, develop 41f30ed); item 4 is a doc line.

## Facts (issue #645, checked on develop 41f30ed)
Probe `o.nika`:

    fn main() {
        let mut a: Vec[i64] = []
        let p = overlap {
            a.push(1)
            a.push(2)
        }
        println(a.len().to_string())
    }

`nikaia check` passes; `nikaia run` fails with the rustc message mapped back: *Cannot borrow `a` as mutable more than once at a time* at 3:5. Expected: `NK2104` from the checker, naming `a`, at the second branch.

Spec (Part I, 10-nikaia-light.md, *The branches meet on nothing*, written this round): *a pair where one branch **changes** a binding (a `mut` method, an assignment, a `mut` argument) that the other names is refused the same way: the message names the binding. Two branches that only read a binding meet on nothing.* ADR-292 D3: `&`/`&mut` are read/write.

Item 2 (`ns.par_iter().map(fn(n) { spin(n) }).collect()` over `Vec[i64]`) runs today and prints `3`; item 3 (`An.` in NK2202) is gone. Do not change them; one pin test for item 2 at both settings if `tests/compiler/tests` has none.
Item 4: `benches/overlap/README.md:59` still names `ordering = "effects"`, which no longer exists: say what the bench measures today (read `benches/overlap/`), in its own commit.

## Locations
* `crates/nikaia-std/src/tools/check_walks.nika:57` `branches_meet_on_nothing`: the bound-name loop, then `accounted(..)` per branch and `check_walks_clash` (line 37). The new check goes here: per branch, the bindings it changes and the names it mentions; a pair where one's changed set meets the other's names (changed or read) gets one `NK2104` per later branch (keep the `break`).
* `crates/nikaia-std/src/tools/order.nika:235` `accounted`, `names_in` (mentions), `reduce` (486, `methods`): reuse `names_in`; a method's `mut` receiver is known from the method table the checker already uses for `mut` borrows (find how `NK` for "changes a binding not declared `mut`" decides it; `nikaia run scripts/where.nika -- NAME`).
* Existing tests: `crates/nikaia/tests/it/overlap.rs` (Rust, do not add there).

## To build
1. The checker refuses the probe with `NK2104`: message *These two branches can't run at the same time.*, a note naming the binding (*both change `a`* / *one changes `a`, the other reads it*). Tests in `tests/compiler/tests/`: `overlap_two_branches_change_one_binding.nika` + `.stderr`; `overlap_one_changes_one_reads.nika` + `.stderr`; `overlap_two_branches_read_one_binding.nika` + `.stdout` (still runs). `lower-std`, commit.
2. `benches/overlap/README.md` line, own commit. Last commit says `Fixes #645`.
Run: `nikaia test --both-settings --project tests/compiler`, `cargo test --profile fast -p nikaia --test it -- overlap:: sysroot`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b645-overlap-shared-mut.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
