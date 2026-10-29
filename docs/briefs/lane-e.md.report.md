Result: partial
Pushed:
Tests: per commit (70f3237c 1ff7291e 0500468e 35ff6933 f9b8b094 df388e6f 705725a1); last gate: tests/compiler --both-settings 348/348, it guard set 23 ok, clippy -D warnings clean, pre-push-check ok, lower-std at fixpoint
Deviation: #915+#910, #930+#940+#918, #970+#937 share commits (one file each); unfixed runs: #908 and #915 tests failed on the unfixed binary, the others are the issues' probes that fail on develop as their issues record (no unfixed rebuild in a shared tree)
Found: skipped as checker-side (lane C): #907 (check_core_method text_in_lists only for Variable/LitStr args), #947 (compared_views/ordered_texts facts), #938 (copied_loop_bindings for an untyped [1, 2], #931), #896 step 4 (the T? result of a filter_map lambda needs wrap facts on its tail and returns), #980 (check_core_call_lent puts a moved text into lent_args before a `return`, so the either param gets either(&t)); #899 step 2 does not reproduce; #925 rest (out.push of a mixed local into Vec[String]) is text_tiers (lane T); #627 case 1 now shows as E0507 (`Pair { a: words[0] }` into a pub struct's owned field moves out of the list), which needs a checker or owner call, case 2 is masked by it; #882 step 1 (lent String? as Option<&str> in emit_function) cannot be tested until the lent-option inference exists (keeps/lends), so build both together; #958 waits only on #938
Needs: lane C for #907 #947 #938 #896.4 #980 #627; lane T for #925 rest; one agent for #882 steps 1+2 together
Changelog: per commit report (#908, #915/#910, #911/#925 part, #913, #930/#940/#918, #899 step 1, #970/#937)
Gate: green
Files: docs/briefs/lane-e.md.report.md
Commit: lane E report: 14 fixes pushed, rest classified
