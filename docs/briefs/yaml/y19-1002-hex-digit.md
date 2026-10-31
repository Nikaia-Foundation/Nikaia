---
agent: builder
model: opus
max_turns: 40
budget_usd: 8
---
# Brief: `yaml` reads hexadecimal digits with the built-in `hex_digit` (#1002 is built)

Score: removes a workaround in `packages/yaml`. #1002 is fixed on develop by 705a5db2 (winnow-grammar 4f8e6691, Part II §10.8): `hex_digit` is one scalar of `0-9a-fA-F`, built like `digit`, with its fixed-width fast path. `packages/yaml/src/main.nika` still matches a hex digit with its own rule, `rule Hex -> i64 = x:any if(is_hex(x)) -> { hex_value(x) }`. It is used by `Hex2`/`Hex4`/`Hex8` (the `\x`, `\u` and `\U` escapes) and by `%` `Hex Hex` in `UriChar`/`TagChar`. This change touches only `packages/yaml` (no version raise, no `CHANGELOG.md`).

Read `docs/builder-facts.md`. Run `git log -1 705a5db2` and read `tests/compiler/tests/grammar_hex_digit_is_one_hex_digit.nika`. It shows what the built-in gives a label (a scalar; over Bytes, a byte).

## To build

1. In the grammar, `Hex` matches `h:hex_digit` instead of `x:any if(is_hex(x))`. Its value stays `hex_value(h)`, or a `hex_digit{2}`/`{4}`/`{8}` form if that reads better and keeps the escape tests' output byte for byte. `UriChar`/`TagChar` take `hex_digit hex_digit`.
2. Remove `is_hex` if the grammar no longer uses it. Its other use near line 1086 (number reading) stays as it is, unless it is the same check and the change is plainly correct.
3. Gate: the package gate (`target/fast/nikaia test --project packages/yaml`, every test output unchanged), `cargo clippy --all-targets -- -D warnings` is not needed (no Rust changed), and `scripts/pre-push-check.sh`.

If `hex_digit` does not do what Part II §10.8 says, or the escape tests change, stop: `Result: blocked`, with `Found:` and a probe.
