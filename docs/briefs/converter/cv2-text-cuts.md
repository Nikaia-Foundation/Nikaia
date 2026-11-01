---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# Brief: rs2nika cuts a text at a number through its bytes

Score: converter, the first of S1's three converter needs (`docs/selfhosting-plan.md` row S1: `sysroot.rs` `shared`, `raw_string`; NK1214 on every `text[a..]`); the same form cost P5e/S2 (`sha256_hex(..)[..16]`). New rule over known Nikaia forms: medium.

Facts: Where things live, Traps already met, Commands

## Problem

Rust cuts a `&str`/`String` at byte numbers (`&text[a..b]`, `text.get(a..)`). Nikaia cuts a text only at a `text::Pos` it handed out (ADR-374 D1; Part I, *A position in a text*; NK1214); a number cuts the text's bytes, and the cut is made text again: `s.bytes[a..<b].text()` (the `docs/selfhosting-gaps.md` row `s[..16]` on text). The converter writes `text[at..]`, `text[copied..<at]`, `text.get(field..)`, `rest[name..]`, each NK1214.

Draft today (`rs2nika --cut crates/nikaia/src/sysroot.rs:884-1044 > s1.rs`, then `--profile check-free --prefix sysroot_package_ --file s1.rs`): `text[at..].starts_with("impl")`, `out.push_str(text[copied..<at])`, `(text.get(field..) ?? "")` after an `unwrap_or_default` TODO, `rest[name..].starts_with(": ")`, `text[at + 1..].scalars.next()`, `text[i + 1..].find(close)`.

## The rule (each Nikaia form below was run on `target/fast/nikaia` at c60babc2)

`t` is a text: a parameter or local the converter types `String` (Rust `&str`, `String`, `&String`), a field typed so in the input or the facts, or a cut of one. A receiver of unknown type is left as it is (a `Vec` slice is right as written).

| Rust | Nikaia |
| :--- | :--- |
| `&t[a..b]`, `&t[a..]`, `&t[..b]` in a function that throws | `t.bytes[a..<b].text()`, `t.bytes[a..].text()`, `t.bytes[..<b].text()` |
| the same in a function that does not throw | `… .text() catch { panic("cut inside a scalar") }` (Rust panics there too) |
| `t[a..].starts_with(x)`, `.ends_with(x)` | `t.bytes[a..].starts_with(x.bytes)`: no `.text()`, nothing to catch |
| `t.get(a..)` | `(if a <= t.bytes.len() { t.bytes[a..].text() catch { null } } else { null })` |
| `.unwrap_or_default()` on a `String?` the converter typed | `?? ""` (today a TODO marker) |
| `&t[..]` | `t` |

A cut followed by a search that gives a number in Rust (`t[i..].find(x)`) is not this brief's: leave it as written with a `TODO(rs2nika): a position, ADR-374` marker.

## Locations

Find the index-expression writer and the `unwrap_or_default` TODO with `nikaia run scripts/where.nika -- --outline crates/rs2nika/convert/src/tr_expr.nika` (index and method-call writers), and the type of a name the converter already knows (the code that writes `ref Vec[String?]` for a parameter, `tr_types.nika`).

## To build

1. The rows above in the tree engine (the default). The line engine only where the same writer serves both.
2. Test `crates/rs2nika/convert/tests/text_cuts.{args,in/f.rs,stdout}`: one Rust function per row, one throwing and one not, plus a `Vec<u8>` slice and an unknown-typed receiver that stay as written.
3. Run the converter on the S1 cut above and put in `Body:` which of the six draft lines changed; check the `shared` draft's cuts with `nikaia check` on a scratch copy only as far as the cuts go (the draft has other gaps: cv3, #1126, #1127).
4. `nikaia build` and `nikaia test` in `crates/rs2nika/convert`; `target/nikaia/bin/rs2nika --corpus tests/corpus.in`, the four score lines into `Body:`; a floor rises only to what this rule earned.

Stop about 15 turns before `max_turns`, commit what is finished, report `partial`.

Commit `rs2nika: a text cut at a number goes through its bytes (ADR-374)`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/converter/cv2-text-cuts.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
