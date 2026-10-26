# Brief: packages/json, #604 - build as specified (owner's correction, 2026-10-26)

The owner rejects 151f9fd where it departs from ADR-344 and where it leaves
things open. Read docs/specification/adr/adr-344.md and issue #604 first.

## 1. Dynamic Value: numbers stay TEXT (owner's decision)
`Value` has no `Int` / `Float`. A number is kept as its text (a view of it,
`Number(String)` or the nearest form the language allows) exactly as written
(`12.0`, `1e3`, `-0`, a number outside i64 all keep their text). It is converted
only when asked for as a type: accessors/functions that read it as `i64`, `u64`,
`f64` etc. and say (throw or `?`) when the text does not fit that type, so the
caller knows what was asked for. `12.0` read as an integer is the caller's
call, not the parser's. The parser still validates the JSON number grammar and
reports line/column/path. Object member order stays text order, duplicates stay.
Update the five tests and add one for conversion (12.0 as i64 and as f64, 1e3,
a number beyond i64, a non-number asked for as number).

## 2. No workarounds (CLAUDE.md "Fix, do not work around")
- No cast `i32/u32/i64 as scalar`: fix in the compiler (a cast or a std function)
  with a Nikaia test, then delete the hand-built UTF-8 byte code in the package.
- `match error { E => ... }` on a struct error lowers to invalid Rust: fix in the
  compiler with a Nikaia test.
Each compiler fix is its own commit with test. Do not touch version lines,
CHANGELOG, badges; name in the report what each change decided.

## 3. Then the To-build list, in order, as far as it carries
Step 2: `json::write[T](value)` (ADR-344 D7, ADR-338 D5), reflected `T::fields`,
tests. Step 3 (building a value from reflected fields in the compiler, then
`json::read[T]`, D3/D4/D8) is the compiler work ADR-344 names as prerequisite:
do it, do not leave it as a note. If a step truly cannot be finished in your
turn budget, stop at a green commit and report exactly what is missing.

Rules: develop only, `git add <files>`, push fast-forward, gate as in CLAUDE.md
(fmt, clippy -D warnings, touched tests, lower-std after .nika changes).
