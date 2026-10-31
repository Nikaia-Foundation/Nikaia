Result: done
Batch: y17-1041-entry-takes-a-parameter.md, y2b-keys-directives-limits.md, y16-1039-unbound-text-names.md
Owner: #1040 (a `return` inside a grammar action reaches rustc): A, it gives the action's value as in a function body (recommended), or B, it is refused? Nothing in yaml needs it.
Pushed: 477879ca
Found: y14's Found filed as #1039 (ready; an unbound text(d:P+) and a name below a repeat read by an action reach rustc). Probing y2b turned up #1041 (ready; a function parameter handed to an entry rule's value parameter stays lent, rustc), which blocks y2b step 2, so y17 runs first, and #1040 (decision).
Changed: #928 To build updated (y2c, y14, y15 done; new order); 95a690fe linked to #928 by a comment; .claude/agents/builder.md: never `Fixes: none` in a report (commit.nika writes it into the message).
Needs: scripts/commit.nika should skip `Fixes: none` as it skips `Body: none` (a script change, for a builder of the main lane).
