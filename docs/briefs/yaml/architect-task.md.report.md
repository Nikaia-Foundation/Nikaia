Result: done
Batch: y11-1032-guard-call-tiers.md, y12-1033-guard-text-label.md, y2c-properties.md
Owner: none
Notes: y2c run 1 blocked on two compiler defects, both reproduced on 506edc2d and filed `ready` in brief form: #1032 (a call in a grammar guard is no call site for the text tiers: `bind_grammar_pattern` drops `Pattern::Guard`, NK1185) and #1033 (a guard cannot read a `text(..)` label that binds names: `s` is bound only in the action prelude, rustc). y2c re-queued (run 1's report renamed `.run1.report.md`; pre-flight now probes both fixes and checks the draft). y2b waits for the next batch. #928's To build updated (labels ready+wip). Rules: architect.md (a draft brief applies the draft and checks it first), builder-facts (a guard is its own case for both passes).
