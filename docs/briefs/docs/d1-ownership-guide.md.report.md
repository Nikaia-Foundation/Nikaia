Result: done
Files: guide/ownership.md guide/syntax.md
Commit: guide: what ADR-views-3 changes, old and new spelling side by side; the tour shows for mut and generic functions
Body: Step 1 is 4242cc99 (guide/ownership.md, syntax.md's memory section shortened to a summary that links it, Where next lists). This commit adds the chapter's last section, What changes next: ADR-views-3 is decided and not built (#1213), said at its top; keep on parameters, self / mut self / keep self (with the note that plain self changes meaning, consuming today, lent after), ref leaving the language outside extern "C", inferred views, a result's source written `-> T from a | b`; today's cart runs as shown, the new spelling is marked does not compile yet and was checked to be refused. The tour gains `for mut` in Loops and a Generic functions section (type parameters, `empty[String]()` at the call), both built and run.
Part: #1213
Pushed: local 54c381c96a716f5dbce8c3a3f8d80157d9d92c55
Tests: every nika block of guide/ownership.md and of the new syntax.md blocks run with target/fast/nikaia, outputs and errors pasted as printed; the does-not-compile-yet blocks checked refused by the parser; links and anchors checked; pre-push green
Gate: green
Deviation: none
Found: (1) Part I 6.8's example shows NK2301 for changing a list inside its loop; the compiler emits NK2305 (as Part III C.3 says) and NK2301 is emitted nowhere; the guide teaches NK2305. (2) defect: `struct Badge { owner: String, number: int }` `fn badge_for(name: String, n: int) -> Badge { return Badge { owner: name, number: n } }` called with owned text (`"grace".to_uppercase()`) is refused NK2302 "hands back a view of `n`", an int; without the int field it runs. (3) defect: `let mut y = "short"; let r = longest(x, y); y.push_str("er"); println(r)` gives NK2105 "`longest` keeps it" instead of NK2305; with `"short".to_uppercase()` it is NK2305 as Part I 6.6 says. (4) Part I 6.6's examples write `"short".clone()`, refused today by NK1301. (5) NK1138 on `fn add_guest(names: Vec[String]) { names.push(..) }` is followed by a spurious NK2105 at the caller naming `Vec[ref String]`.
Needs: none
Changelog: none
Owner: none
Disk freed: none (target/nikaia-project-tests 2.7G, / 9.1G free, no new cache entry over 100 MB)
Rs2nika: none

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
