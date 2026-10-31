Result: done
Batch: bt-build-time.md, s922-sha256-env.md, gr-grammar-run.md
Pushed: 2176ff2d
Files: docs/briefs/{batch.txt,bt-build-time.md,s922-sha256-env.md,gr-grammar-run.md}, docs/briefs/bugs/{batch.txt,b8-1044-last-view-handed-back.md}, docs/architect.md (State)
Issues: #1044 gets `wip` (moved to the bug lane as b8); #922 gets a comment with the steps 1-2 decision (SHA-256 written out in hash.rs, no sha2 in std per ADR-002 D4; env::var has no touches line, which means it orders against everything)
Found: b1046's NK2302 "settings disagree" did not reproduce: a_lent_argument_from_arms_view_and_new passed at both settings on the 042cee5d binary; nothing filed
Changelog: b1046's line is owed for the version raise (in the State section)
Owner: none
