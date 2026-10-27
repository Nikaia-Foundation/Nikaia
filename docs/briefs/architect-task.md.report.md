Result: done
Batch: b-fixforward-range-guard.md, b893-commit.md, b894-lent-argument-arms.md
Owner: none
Notes: RED since f74c199: the range_spelling guard flags ".." inside string literals of a `for` head in rs2nika/convert/src/main.nika (a false positive); fixed in the guard, not in the converter lane's file. b893's verified work stays uncommitted in the checkout until b893-commit.md commits it (Fixes #893). Committed d7ac87e.
