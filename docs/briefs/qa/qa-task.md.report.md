Result: partial
Cause: 9ebcd115 (spec: ADR-389 `immutable`) added two blocks to Part I, so the block numbers in EXPECTED.txt and COMPILES.txt shifted: specification::the_specifications_programs_are_the_ones_in_expected_txt and what_lowers_is_handed_to_rustc_and_the_verdicts_are_the_recorded_ones
Cause: 9ebcd115 also added `immutable` to the reserved-word list on the page, not to the parser: from_is_a_name::the_page_and_the_table_list_the_same_words (still red)
Fixed: regenerated tests/specification/EXPECTED.txt and COMPILES.txt. Without the block numbers the old and new files differ only by the two new fragment blocks, so no verdict changed.
Tests: `cargo test -p nikaia --test it -- specification::` 5 passed. from_is_a_name not rerun after my change: it needs the parser change below.
Found: the build of ADR-389 is missing. `immutable` is not reserved in the grammar (RESERVED_A..C in crates/nikaia/src/parser/mod.rs) or in RESERVED_WORDS (tools/parse_errors.nika, 38 words). The compiler's own .nika uses `found.immutable` and `local.immutable` as field names. Adding only the word to the list would be a workaround. Probe: `nikaia run` on `fn main() { let immutable = 1 }` parses, but the page says it is reserved. Needs: supervisor decision, either file the build of the construct (the build reserves the word) or hold the page's list until the build lands.
Files: tests/specification/EXPECTED.txt tests/specification/COMPILES.txt
Commit: Specification baselines follow the two blocks ADR-389 added to Part I
Changelog: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 177d2531465658e38f6f82c41909e6011327ed55
