---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: #872, #873, #874: three defects found writing the guide's Memory section

Bugs lane. Read each issue on GitHub (nikaia-foundation/nikaia) for the probe, the actual output and the To build list. Order: #874 (small, messages and spec), #873, #872 (largest).

* **#874**: the NK2105 note says "`char`s"; Nikaia has no `char` (Part I `10-nikaia-light.md` :276). Fix every message listing `char`s (`grep -rn "char" crates/nikaia/src crates/nikaia-std/src/tools/*.nika` in diagnostics), Part I lines ~1695, ~1902, ~2384, and the `.stderr` files. Commit `Fixes #874`.
* **#873**: `println(struct)` / `f"{struct}"` with no display reaches rustc. Refuse it in the check. **Reserved codes: NK1302, NK1303** (use NK1102 instead if the existing mismatch message fits; say which in the report). Commit `Fixes #873`.
* **#872**: a `for` element (`ref T`) passed to a `T` parameter the callee only reads is NK1102; Part I :1307-1311 and :2648-2650 say it is lent and a view is passed as is. Accept it; keep NK1102 for a keeping parameter. Commit `Fixes #872`.

Tests: Nikaia only, `tests/compiler/tests/*.nika` with `.stdout`/`.stderr`. Do not touch version lines, CHANGELOG or badges; name in the report what each change decided. Lowered files are regenerated, never edited by hand. Gate per `.claude/agents/builder.md`.
