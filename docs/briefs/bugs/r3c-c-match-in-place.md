---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Fix the next two defects behind chunk 2 of type_rules (#125)

Blocks: type_rules chunk 2 (the self-hosting move on wip/r3c-type-rules-chunk2, 6465e74c)

Source: docs/briefs/bugs/r3c-a-taking-arms.md.report.md (Found lines); the
wip is on branch wip/r3c-type-rules-chunk2 (6465e74c). Do not merge or move
that branch.

Facts: Tests and the gate, Probes, Checker and views, Lowering and lower-std, Moving Rust into Nikaia

Fix, each with a test in tests/compiler/ and as its own commit:
1. #1139: a `match` over a place inside a `mut` parameter whose arm changes
   a part in place lowers as a move (E0507); `tr_rewrite_item`'s
   `match item.node { Item::Impl { ref target, methods, .. }` is the case
   the wip needs.
2. #1140: a taking arm that gives the value back on only one branch is
   accepted, and `s = if c { A } else { B }` is refused with NK2108.

In the report, under Needs: whether the wip's three files now build (try
them in a scratch copy, never in the shared tree).
