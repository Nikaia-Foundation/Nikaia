Result: done
Batch: b4-1016-list-view-result-slice.md b5-1002-hex-digit-built-in.md b6-1010-let-mut-literal-owned-field.md
Owner: none
Red: e064b36e (b1, #999) failed arm64 Test once; 832b07dd, which contains it, passed: a flake (NK2302 under concurrent runs, as b1/b2 reported), no fix-forward.
b3-1016 partial: decided emit_core_types.nika belongs to the fix and ref Vec[T]? follows the same rule; patch and test draft copied to docs/briefs/bugs/b4-1016.* (b4 removes them). Comment on #1016.
Labels: wip set on #1002, #1010 (#1016 kept); wip taken off #939 (b6c7c3b7 Fixes #939, closes on promote).
Probed on target/fast/nikaia (08:38): #1002 "Undefined rule: 'hex_digit'", #1010 rustc Mismatched types, #1016 nullable case rustc E0308.
Next candidates: #1014 (interface key misses inferred sync; a builder wrote around it), #1038, #1001, #1004.
