Result: done
Batch: b1012c-trait-column-data.md, b1017-lambda-optional-tail.md, b1018-kept-fn-index-read.md
Owner: none

Decision: b1012b (51b0838a, #1012 step 2) is done; its Changelog line is in its report and the version raise is still owed. Its Found is not a defect: `nikaia fmt` puts tabs into a tools file because Part III says it must. The rule now sits in builder-facts as "never fmt tools files". Its Deviation, an existing test's stderr updated because the change rightly altered the message, no longer counts as a deviation (common.part). CI is green up to df143a1b.
b1012c now writes `Fixes #1012`: step 4 (k3a2, 829ce635) is on develop. #1017 and #1018 were probed at ed30efd5 and still fail. #1017 traces to `check_core_lambda_leave`. #1018 drops the issue's `fs[0](2)` row because it does not parse. I commented on both issues and committed in 8de92b2c.
Needs from the main session: nothing.
