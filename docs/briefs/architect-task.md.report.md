Result: done
Batch: b-fixforward-script-tests.md b866-inferred-number-local.md
Owner: none
Notes: develop red since 6370390 in bootstrap_compiler and arm64: four script tests hard-code target/fast/nikaia, which CI does not build. Fix-forward first. #863 closed by hand (noop). A noop's reason now goes under Tests: (common.part); builder-facts has "CI has no target/fast".
