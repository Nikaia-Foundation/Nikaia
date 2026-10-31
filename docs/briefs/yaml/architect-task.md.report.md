Result: done
Batch: y8-1026-grammar-label-struct.md y2c-properties.md y2b-keys-directives-limits.md
Owner: none
Decided: y2 stopped on a compiler defect, filed as #1026 (ready): a grammar label binds its local without the struct it holds (`bind_grammar_pattern`, text_tiers.nika), so a view field read through it counts as owned text (NK1106); the function-call path (#803) works. y8 fixes it (`Fixes #1026`, crates/, Changelog owed); y2c resumes y2 step 2 from the draft on wip/y2-properties (applies cleanly on develop); y2b unchanged but for its reference. #928's To build updated. architect.md: a grammar brief probes its tree shapes across rules (the y2 lesson).
