# E10-0: call graph of crates/nikaia/src/emit/mod.rs (9594 lines)

Method: script over `fn` spans (line to next `fn`); callee = name followed by `(` in the body.
Counts are approximate (nested/free fns folded into the span above them).
Found 168 `fn`s in `impl Emitter` and below (line 1826 on), not 215; the rest are
free fns, `Needs`, `SourceMap`, `Build`, `Target` (lines 1-1800, small).

## Layers, leaves first
* Leaves (no callee in the file): 96 fns, about 1450 lines (escaped, base, unary_op,
  binary_op, repeat_suffix, integer_literal, pointing, frame_attribute, tuple_pattern ...).
  Move first, with the free helpers 8653-9560.
* Mid: `text`, `ty_filled`, `new`, `walking`, `find`, `pattern`, `block`, `expr`, `stmt`
  (the recursive core `a_stmt` 6265, 939 lines, and `expr_written` 7547, 662 lines,
  43 and 25 callees; one step, see selfhosting-plan).
* Top: `function` 3726 (1113 lines, 24 callees), `item` 2690 (634, 21), `program` 2449 (146, 10).

## Callers of the six entry points
* `grammar` (5060) <- `item` only. `fold` (5416) <- `pattern_body` only.
* `function_body` (4839) <- `function` only. `function` <- `item`, `error_impl`.
* `item` <- `items_only`, `program`. `program` <- nobody inside the file (public API
  `emit_program*`, 679-720, calls it).

## Chunking E10-1 onward (lines, from the spans)
| Step | Functions | Lines |
| :--- | :--- | ---: |
| E10-1 grammar | grammar 20, fold_options 35, grammar_over 12, grammar_body 51, grammar_rule 79 (5060-5260) | ~200 |
| E10-2 fold | fold 44, fold_step 54, fold_merge 67, in_place_fold, fold_callees 4 (5416-5600, 8856) | ~230 |
| E10-3 function_body | 212 (4839-5060) plus its private helpers | ~250 |
| E10-4 function | 1113 (3726-4839); split in two if it passes 120 turns: signature/header, then body wiring | ~1100 |
| E10-5 item | 634 (2690-3726 minus function) | ~640 |
| E10-6 program | items_only 88, program 146, folded 7, with_contracts 389 (1855-2450) | ~630 |

Prerequisite for every step: the callees below it already in Nikaia (leaves and the
`a_stmt`/`expr_written` core). If the core is not moved, each step takes callbacks
(`fn(ref Expr) -> i64 sync`, as `bounds_basic.nika`).

## Name clashes (one global namespace for tools/*.nika)
No `.nika` file defines `function`, `item`, `program`, `grammar` or `fold` today
(grep of `fn|struct|enum`), but the check tools use such words as parameter and local names
and the check side will want them. Prefix every moved name `emit_`:
`emit_program`, `emit_item`, `emit_function`, `emit_function_body`, `emit_grammar*`,
`emit_fold*`. `emit_program` also exists as a Rust `pub fn` (679): the adapter keeps
that name, the Nikaia one becomes `emit_program_items` or the Rust one calls
`nikaia_std::tools::emit_program::...` by module path. Decide in E10-6.
`grammar` and `throws` are reserved words (#799): never a parameter name.
