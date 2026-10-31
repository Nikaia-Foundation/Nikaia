Result: done
Batch: b20-1088-tuple-option-part-in-arms.md, b21-1097-flat-map-seq-result.md, b13-1057-field-of-a-coalesce-and-let.md
Owner: none
Stop: CUT on b18's Found, not a red CI (b18 #1078 done, 46824f2b local; suite still running on HEAD).
Found filed: #1097 flat_map with a Seq result, NK1245 (ready, wip; decision on the issue: a Seq/Vec/Array result binds U, lowering unchanged); #1098 named fn and filter_map if/null over iter() of copy elements reach rustc (ready, next after this batch). All three rows re-probed on target/fast at 0510bf6f.
Labels: #1078 keeps wip until promote closes it; #1088, #1057 keep ready,wip.
Next after this batch: b14 #947, b16 #1065 (briefs written), then #1098, #1081.
Pushed: local 42c74e6e, not pushed: HEAD carries the untested b18/s1082/GR commits the running suite gates; the driver pushes them together.
