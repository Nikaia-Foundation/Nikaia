---
agent: builder
model: opus
max_turns: 200
budget_usd: 30
effort: medium
---

# Issue #{N}

Your work order is issue #{N}, inlined below with its comments. There is no
brief written around it: plan your own steps from its *To build* list, build
them in order, commit after each finished step, and end when its *Done when*
holds (or at the limits of `builder.md`, reporting what is left). The
facts sections in your prompt come from `docs/builder-facts.md`.

Measure the issue against `docs/issues.md` while you work: what it lacked,
what was wrong in it. That answer goes into the report's `Issue:` field.
A finding, a changed plan or a finished item also goes into the issue as a
comment (`gh api repos/Nikaia-Foundation/Nikaia/issues/{N}/comments -F body=@file`,
ending with the Claude Code footer), rule I13.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the issue names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `{REPORT}`)
Fields: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Issue, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Result: done` only when the issue's *Done when* holds; `partial` with the items left otherwise.
* `Deviation:` only for an item of the issue built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else goes into `Body:`; how you verified goes into `Tests:`.
* `Issue:` by `docs/issues.md`, *How a run judges its issue*: `complete`, or `broke I<n>: ...`, `rules-gap: ...`, `wrong I<n>: ...`, joined with `;`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
