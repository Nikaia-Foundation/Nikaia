Result: done
Batch: y4c-1003-finish.md, y7c-match-join.md, y2-tree-properties.md
Owner: none
Stop: CUT on y7b's Found (y7b itself done, gate green, 65d86e57, Fixes #1009). The `RED` file in this dir is an empty driver marker (gitignored), not a red CI.
Found filed: #1021 (label ready): a `match` arm's assignment to a `let mut` is not joined as after an `if`. A defect: ADR-361 D9 says "a branch". Its fix is y7c (Fixes #1021). It touches only prover files, so it runs after y4c.
y2: cut to steps 1-2 (block scalars, properties, aliases), 100 turns. Steps 3-4 (explicit keys, directives, Limits) moved to y2b-keys-directives-limits.md, for the next batch. Both now carry the stop-15-turns rule.
#928: To build updated (y7b struck, y4c/y7c/y2/y2b listed, #1021 under Found); labels stay ready,wip.
Pushed: 35f48c51
Not mine, left alone: packages/yaml/nikaia.lock is modified in the tree (compiler hash), and the old report was deleted by the driver.
Process: y7b's debrief asks builder.md for `cp target/fast/nikaia $S/nikaia.before` before the first build, and for clippy grep -A3. I put both into y7c. builder.md belongs to the main session: worth adopting there.
