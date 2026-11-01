---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Fix the two defects that block chunk 2 of type_rules (#125)

Source: docs/briefs/r3c-type-rules-direct.md.report.md (Found lines); the
probes are on branch wip/r3c-type-rules-chunk2 (6465e74c). Do not merge or
move that branch.

Fix, each with a test in tests/compiler/ and as its own commit:
1. A taking arm over an enum declared in another file (ast Expr/Item) gets
   no placeholder: CheckState.a_placeholder (check_items.nika) reads only
   view.items' enums, and a NodeId field does not count as defaulted. The
   lowering then moves parts out of &mut (E0507).
2. A Copy binding from `match open(base)` on a mut place is compared as
   `b == *binding`, giving E0277 `&mut Symbol == Symbol`.

Another builder fixes lambda defects in parallel in the same checkout.

In the report, under Needs: whether the wip's three files now build (try
them in a scratch copy, never in the shared tree).
