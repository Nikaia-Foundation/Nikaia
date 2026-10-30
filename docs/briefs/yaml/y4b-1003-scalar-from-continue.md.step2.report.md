Result: partial
Files: packages/json/src/main.nika packages/json/nikaia.derived scripts/grammar_arrow.nika
Commit: json, grammar_arrow: a character written as one, not a literal cast as scalar
Body: `0 as scalar`, `8 as scalar` and `12 as scalar` become `'\u{0}'`, `'\u{8}'` and `'\u{c}'` (#1003 step 2). json's `\u` decoding keeps `cp as scalar`: `scalar::from(cp)` there is not proved (hex4's range across its loop is unknown to the prover, and `cp` reassigned in a branch is not read), so it waits, as does step 3's refusal.
Part: #1003
Changelog: none
Pushed: rejected
