---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Fix two lambda defects the r3c move found (#125)

Source: docs/briefs/r3c-type-rules-direct.md.report.md (Found lines);
probe p20 is in /tmp/claude-0/-home-user-Nikaia/b7e2d91b-1a15-5371-a36e-8c201aeab2d2/scratchpad/r3c/.

Fix, each with a test in tests/compiler/ and as its own commit:
1. An enum variant with a fn field, built from a lambda or a named fn,
   lowers to mismatched types (Kept<dyn Fn> against a closure).
2. `Vec[String].contains` inside a lambda yields &'static str.

Another builder fixes taking-arm defects (check_items.nika placeholder,
Copy compare under a mut-place match) in parallel in the same checkout;
leave those files alone where you can and name any overlap in the report.
