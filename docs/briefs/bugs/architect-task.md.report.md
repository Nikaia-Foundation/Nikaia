Result: done
Batch: b5-1002-hex-digit-built-in.md b6-1010-let-mut-literal-owned-field.md b7-1016-named-lifetime-and-typed-let-slice.md
Owner: none
Notes: b4-1016 (partial) built #1016 step 1 (6832794b, on develop). Its step 2 (nullable) re-probed: a nullable parameter returned as is runs today; only a plain view returned as a nullable result fails, now #1016 step 4 (comment on #1016). b4's Found (named 'r lifetimes keep &'r Vec<T>; typed let) is b7. A function type with two view parameters has no lifetime (also for text): filed #1048 (ready). b5/b6 probes re-run on 3adb2b60, still failing as written. wip held: #1002, #1010, #1016. Commit a10b21ff.
