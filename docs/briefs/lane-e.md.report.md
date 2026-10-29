Result: partial
Pushed:
Tests: either_text_reassigned (new, the #911 probe; failed unfixed per the issue, rustc mismatched types), either_text_compared_with_owned (new, the compare of the #925 probe; failed unfixed per the issue: can't compare String with EitherText); tests/compiler --both-settings 348 passed; it guard set 23 passed; nikaia-std --lib either_text 2 passed
Deviation: #925 is only half fixed: `out.push(line)` of a mixed local into a `Vec[String]` still fails (the local is typed `EitherText` by text_tiers.nika, lane T's); the issue's full probe is not committed as a test
Found: #925 rest: text_tiers.nika should own a mixed local where it goes into an owned position (push into Vec[String])
Needs: lane T for #925's rest
Changelog: A `String` place assigned the result of a function that answers text of both kinds takes it as text of its own (`.into()`, #911), and a `String` compares with such text (#925, part). Decided: runtime `From<EitherText> for String` (moves an owned answer, copies a view) and `PartialEq<EitherText> for String` in nikaia-std's either_text.rs.
Gate: green
Files: crates/nikaia-std/src/tools/emit_below.nika crates/nikaia-std/src/tools/lowered/emit_below.rs crates/nikaia-std/src/tools/emit_stmt.nika crates/nikaia-std/src/tools/lowered/emit_stmt.rs crates/nikaia-std/src/either_text.rs tests/compiler/tests/either_text_reassigned.nika tests/compiler/tests/either_text_reassigned.stdout tests/compiler/tests/either_text_compared_with_owned.nika tests/compiler/tests/either_text_compared_with_owned.stdout docs/briefs/lane-e.md.report.md
Commit: Own an EitherText result assigned to a String place; compare String with EitherText
Fixes: #911
Part: #925
