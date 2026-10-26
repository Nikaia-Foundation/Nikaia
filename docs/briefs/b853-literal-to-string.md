---
agent: builder
model: sonnet
max_turns: 90
budget_usd: 12
---
# Brief: refuse `.to_string()` / `.clone()` on a text literal (#853, NK1301)

**Blocked by #856** (ADR-282 D31: a literal where text of its own is needed is owned). Without it, dropping `.clone()` makes `it::handed_over`, `it::result_source`, `it::nullable`, `it::lent_arguments` fail. `ready` once #856 lands.

## Decision (owner, 2026-10-25: option A, both calls refused)
ADR-282 D30; Part I 2.3 (sentence after NK1189); Part III code table `NK1301` (commit 23de2754). A text literal already fills any `String` slot (Part I 2.2), and by D8 `.to_string()` of a view is the view, so `"-p".to_string()` and `"-p".clone()` mean the bare literal. Generic `x.to_string()` and calls on a value that is not a literal stay.

## Facts
* Today both `v.push("-p")` and `v.push("-q".to_string())` compile and run with no warning.
* About 51 `"…".to_string()` sites in 15 files, plus `"…".clone()` sites (e.g. `check_values.nika:200`). Biggest: scripts/agent.nika 17, tests/language/src 22.

## Locations
* NK1189 refusal: `crates/nikaia-std/src/tools/check_values.nika` ~175-200 (add NK1301 beside it); code list `check_members.nika:58`.
* `tests/compiler/tests/to_string_of_a_view_fills_a_string.nika` (D8 test).
* rs2nika: `crates/rs2nika/convert/src/`.
* Known facts: read `docs/builder-facts.md`. Locations by `nikaia run scripts/where.nika -- NAME`.

## To build
1. Done in a patch, not committed: `/tmp/claude-0/-home-user-Nikaia/84cbb105-595e-5391-82e1-8947de490af1/scratchpad/b853/b853-wip.patch` (checker refusal NK1301, test `literal_to_string_refused.nika` + `.stderr`, about 270 files migrated, rs2nika). After #856: rebase it onto `develop` (`git apply --3way`; re-run the migration grep for sites added since: `grep -rnE '"[^"]*"\.(to_string|clone)\(\)' --include=*.nika crates scripts tests packages examples`).
2. Where a migrated site still fails, the compiler is wrong under D31: report it (`Found:`), do not put `.clone()` back.
3. `cargo build --profile fast -p nikaia`, `target/fast/nikaia lower-std`, commit the lowered diff; `nikaia test` on tests/compiler and the touched `it::` tests (the four above, `text_tiers`, `typecheck`). Last commit `Fixes #853`.
## Rules (CLAUDE.md)
* `develop` in the main checkout; `git add <files>` only; push fast-forward, no rebase (supervisor rebases).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`.
* No version lines, `CHANGELOG.md`, badges. NK code: only `NK1301`.
* New tests in Nikaia only. Never the full suite locally.
* A compiler defect on the way: fix with a test, or file a `ready` issue; no workaround.
* Do not touch other agents' uncommitted files (`git status` at start).
* `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/`; delete `target/release/nikaia-scratch` after tests; report GB freed. Scripts in files, small reads, no foreground sleep.

## Report (at most 15 lines, to `<brief path>.report.md`)
Result, Files, Commit, Fixes, Pushed, Tests, Deviation, Found, Needs, GB freed, Owner (form in `docs/architect.md`, *Briefs and reports*).

## History
Owner asked why Nikaia code writes `v.push("-p".to_string())`. The answer: a Rust habit from rs2nika and hand ports, and before D8 the tier walk did not always let a literal fill a `String`. Options were refuse (A), a warning only (B), or leave it (C). The owner chose A, with both calls refused.
