---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: a `match` over `error` that names a struct error type by its name (#673)

Found while reviewing #673 step 1 (commits `std::regex: run-time patterns…` and `keeps: a parameter of a generic type…`, local on `develop`, **not pushed**). Part I, *Telling failures apart*: a `match` over `error` "is complete when it names every type that can arrive: each variant of it, or the type by a bare name". For an error type that is a `struct` this does not hold.

## Facts (probes on the current `target/fast/nikaia`, built from this tree)
* **A: `std` struct error type by its path.** `regex::dynamic` has `throws = ["regex::Error"]` in `std.contracts`.
  ```nika
  use std::regex
  fn main() throws {
      let re = regex::dynamic("(") catch {
          match error {
              regex::Error => { println(f"bad pattern: {error}"); return }
          }
      }
      println(f"compiled {re.is_match("(")}")
  }
  ```
  is refused with `NK1151` ("doesn't handle `else`") though `regex::Error` is the only type that can arrive. With an added `else => { … }` arm it passes the checker and **rustc** fails: *Expected unit struct, unit variant or constant, found struct `regex::Error`* - a rustc message reaching the user.
* **B: a program's own struct error type.** `struct Bad { why: String }` with `impl Error for Bad`, thrown by `risky`, caught by `match error { Bad => { println(f"bad: {error}"); 0 } }`: accepted and runs (`bad: negative`), but rustc warns *Convert the identifier to snake case*: the lowering writes `Bad` as a Rust binding pattern, which matches anything. It works only because one type arrives; with two types the second arm would be unreachable in Rust.
* A variant pattern (`ConfigError::NotFound(p)`, Part I's example) and `else` are lowered correctly today; follow how the error set reaches the `match` there (ADR-280's envelope) and lower a type named by its bare name or its path as the arm for that member of the set.

## To build
1. **Checker:** a bare type name or a type path (`Bad`, `regex::Error`, `io::IoError`) as an arm's pattern in a `match` over `error` covers that member of the error set; the `match` is complete when every member that can arrive is covered (Part I). A name that is not a type in the set keeps its existing refusal.
2. **Lowering:** such an arm is lowered as the arm for that member, never as a Rust binding or a Rust path pattern; no rustc message or warning reaches the user.
3. **Tests in Nikaia** (`tests/compiler/tests/`, `.nika` + `.stdout`): a program struct error caught by its name; two program struct error types arriving at one `catch`, each named, the right arm taken for each (no `else`); `regex::Error` caught by its path without `else` (probe A); one with a type that cannot arrive, keeping today's refusal (`.stderr`). Run each once against the unfixed compiler (stash the fix) and say in `Tests:` that it failed there.
4. **Rewrite `tests/compiler/tests/regex_a_bad_pattern_is_a_regex_error.nika`** without its wrapper function: `regex::dynamic("(") catch { match error { regex::Error => … } }` directly in `main`, the `.stdout` unchanged.

## Done when
`target/fast/nikaia test --both-settings --project tests/compiler` (all, it is a `check/`/`emit/` change); the guard set of `.claude/agents/builder.md`; clippy clean; `lower-std` at its fixpoint. One commit per step that stands alone (the fix with its tests; the regex test rewrite), `Part of #673`. **Commit, do not push**: the supervisor pushes the step-1 commits and these together. Changelog: one line under `Changelog:`.

## Stop and report if
The error set's representation cannot tell two struct types apart without a design choice (report it under `Owner:`); a `.stderr` outside your tests that would change; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: the two untracked `docs/briefs/b673-regex-dynamic.md*.report.md` files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; **no push** (this brief's exception).
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite beyond `tests/compiler`. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=4G`; delete `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b673b-catch-by-struct-type.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone), Files, Commit, Body (optional), Fixes, Pushed (`no, by the brief`), Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, `Rs2nika: none`. `none` where nothing, as the bare word. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...`).
