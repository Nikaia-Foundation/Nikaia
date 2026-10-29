Result: partial (6 of 11 assigned fixed; the rest open, see Needs)
Pushed: 282dc867 fa7ba00d 5fa2a7a1 9dd789bc 5162370e 2d6c1344 (develop, fast-forward)
Tests: tests/compiler --both-settings 348/348 (898, 936), then 351/352 (909, 917, 931; the one failure was my own test using print, fixed to println); guard set + lent_arguments:: green; borrowed_subject:: 9/9 after 2d6c1344; clippy -D warnings clean; pre-push-check ok; lower-std at its fixpoint
Failed unfixed: 898 NK1102; 936 NK1131 x3; 909 rustc E0308; 917 rustc E0271; 931 NK1245 x2
Deviation: commits made with git commit and hunk staging (git apply --cached) rather than scripts/commit.nika, because emit_expr.nika/.rs, lowered.rs, bounds_walk.rs and check_numbers4.rs also held other lanes' unfinished hunks; only my hunks went in
Changelog 936: a text field of a borrowed subject handed to a parameter whose contract keeps nothing of it is lent, not a move (no NK1131); a keeping parameter still refuses it
Changelog 898: a proved `ref T?` parameter reads as `ref T`; an option bound by reference out of a lent subject reads through `.as_ref().unwrap()` and goes back into a `ref T?` slot as `Option::as_ref(x)` (narrowed_reads bit 32)
Changelog 909: a tuple literal handed to a kept generic parameter is checked part by part; a lent part is NK1102 with the `.clone()` help
Changelog 917: the element/key/value argument of push/insert/contains/remove/get/contains_key/entry asks an open integer for the receiver's type; only Vec/set `contains` needed it; the toolchain's lowered code now writes `: i64` on such locals
Changelog 931: `sum`/`product` of a walk over a list of number literals is that open number (asked by `let total: i64`, else the default i64, written on the let)
NK codes: none used (NK1308-1311, NK2706-2707 still free)
Found 900: lane T's file. text_tiers.nika `tier_struct_of` has no `Expr::MethodCall` arm for `.clone()`, so `let mut c = a.clone(); c.name = t` leaves the field `&'static str` (NK1105). Fix: one arm, `clone` with no args -> `tier_struct_of(receiver)`
Found 2d6c1344: 282dc867 (#936) turned develop red: borrowed_subject's `takes(s: String)` ignored `s`, so it is now a reader; the test now keeps `s` (the new behaviour is right)
Found laneT-1: `let mut sg = s.get("b") ?? return; sg.push_str("?")` does not reproduce at 5162370e (prints `p! q?`); recheck against lane T's tree
Found laneT-2/#980: `[[], ["q"]]` into Vec[Vec[String]] lowers as Vec<Vec<&'static str>>: a push through `lists[0]` is not linked in text_tiers (source_of/place_of lack Expr::Index; #980's own body). Tier pass, not check_core_args owned_texts; lane T's
Found p980 (lane E's probe): `bind(t, w)` where `bind` keeps `name` into `w.items` (a mixed-tier Vec) is lowered `either(&t)` on the path before `return`; lending a local into a kept mixed slot that outlives it needs a decision on when check_core_call_lent may lend to a kept position. Not fixed
Found 927: an index read of an owned list (`out.push(rest[i])`, String) is not covered: check_core_a_lent_place_given_away returns for unlent roots and for all text; telling owned from literal-only text needs the tier result. Not started
Needs: #927 #591 #919 #677(step 1) #968 #907 #947 #938 #896(step 4) #627 #980 are not started (turn budget); a fresh lane C can take them from here
Gate: green
Files: docs/briefs/lane-c.md.report.md
Commit: lane C report: #936 #898 #909 #917 #931 and the borrowed_subject fix
Cache: target/nikaia-project-tests 2.1G; disk / 9.3G free
