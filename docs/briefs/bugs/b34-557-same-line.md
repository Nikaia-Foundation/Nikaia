---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
effort: high
---
# Brief: #557 step 3, the grammar language gets `same_line`

Blocks: the parser grammar port (#557 step 4: `parser/mod.rs`, 2,728 Rust lines, the largest file left in Rust)

Score: decided (ADR-339 D2, Part II 10.8 table row `same_line`); steps 1-2 are on develop (2ca9841, 9545797). One built-in, checker and lowering.

Facts: Tests and the gate, Probes, Grammar, Lowering and lower-std

## Decided
* ADR-339 D2 (copied): "`same_line` matches nothing and fails where the whitespace skipped just before it held a line break. The generated parser records, by position, what its last `WS` skip held, so the answer is a fact about the input and survives backtracking." ADR-339 D3: no *grammar* declares a state or an `extern rule`; the **lowering** may write both into the generated `grammar!`.
* Only a grammar that writes `same_line` (and declares no rule of that name) changes; every other generated grammar lowers byte for byte as before.
* A line break anywhere in the skipped run counts, inside a block comment too. The Rust `Trivia` counts only breaks outside comments, so `a /*\n*/ [0]` is an index there and not here. Name this difference in `Body:` (the port compares on every tests/compiler program).

## Facts
* The Rust model (`crates/nikaia/src/parser/mod.rs`, grep, lines drift): `fn same_line` (`i.current_token_start()`, then `Trivia::began_a_line(at)`), `state Trivia;`, `extern rule same_line -> ();`, the recording `rule WSE = w:multispace1 @ at -> { _state.user().ran(at, w.contains('\n')) }`, the entries building `ParseInput::<Trivia>` with `ParseContext::<Trivia>`. The record is `PrTrivia::ran` / `began_a_line` in `tools/parser_leaves.nika`. The `winnow-grammar` source (`state`, `extern rule`, `_state.user()`) is under `~/.cargo/git/checkouts/winnow-grammar-*/`.
* Where the generated parser is written: `emit_grammar.nika` (`emit_grammar_over`, `emit_grammar_body`); step 2's template rules `__nikaia_span` / `__nikaia_here` and `emit_writes_a_built_in` are the pattern. The checker side: `check_grammar.nika` beside `span` / `here`.
* The lowering writes, in a grammar that uses `same_line`: (a) a `state` of one small record type in `nikaia_std::grammar` (fields `end`, `newline`; `ran`, `began_a_line` as `PrTrivia`'s; one type, no second copy of the logic); (b) a `WS` that records the run it skipped (the grammar's own `WS` renamed under a recording `WS`, or the default whitespace written out the same way); (c) a template rule `__nikaia_same_line` (an `extern rule` over a hand-written function after the block); (d) entries building the parse input with that state. Over `Bytes` and text alike (needs no `text::Pos`, so no NK1187).

## To build
1. `same_line` in the checker (arity 0, binds nothing; a grammar's own rule `same_line` wins) and in the lowering as above. Tests, each a `.nika` with its `.stdout` in tests/compiler/tests/: `grammar_same_line.nika` (`postfix = name (same_line "[" n "]" | same_line "(" n ")")*` over text and over `Bytes`: `a\n[0]`, `f\n(x)` are not an index / call, `a [0]`, `f (x)` are; a backtracking case: an alternative that skips the break and fails, then one asking `same_line` at the same place gives the same answer) and `grammar_same_line_own_ws.nika` (own `rule WS = (multispace1 | "//" until(line_ending) | "/*" until("*/") "*/")*`: `a // c\n[0]` not an index, `a /* c */ [0]` an index, `a /*\n*/ [0]` not an index). One commit with the lowered files, `Part of #557`.

## Done when
tests/compiler with both settings (emit_* changed), `it` tests filtered on `grammar`, the builder gate, clippy, lower-std fixpoint.

## Stop and report if
`winnow-grammar` cannot carry a `state` in a generated grammar over `input bytes`, or its entries cannot be handed one (say what it offers); an existing test's `.stdout`/`.stderr` changes; a change in `grammar_run.rs`, `build_time.rs` or `parser/mod.rs` is needed.
