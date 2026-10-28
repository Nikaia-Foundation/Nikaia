Result: done
Files: crates/nikaia/src/contracts/keeps.rs tests/compiler/tests/regex_a_parameter_searched_with_find_is_lent.nika tests/compiler/tests/regex_a_parameter_searched_with_find_is_lent.stdout
Commit: keeps: a parameter of a generic type narrows its method by the type's name (#673)
Body: The keeps walk left a receiver whose declared type has type arguments out of its type table, so `re.find(s)` on `re: regex::Regex[regex::Dynamic]` was asked of every `find` in the ledgers, `Seq::find` among them, which keeps its receiver: a function that only searched `re` was taken to keep it (NK2105 at the caller's next use). The test failed so on the unfixed compiler. lower-std unchanged.
Part: #673
Pushed: rejected
Tests: tests/compiler --both-settings 215 passed (10 new regex_*); it range_spelling:: shipped_ledger:: contracts:: dependency_ledger:: keeps:: lending:: own_methods:: bytes_owner:: tether_keep:: accepted_compiles::; nikaia --lib contracts::; clippy clean; pre-push-check ok
Gate: green
Deviation: two commits (145faff1 std::regex steps 1-4, report docs/briefs/b673-regex-dynamic.md.step1.report.md; this one the keeps fix); both pushes rejected (develop moved), not rebased per the rules; Cargo.lock also moved errno, rustix, terminal_size to windows-sys 0.61.2 (cargo's resolution, Windows only)
Found: keeps.rs dropped generic-typed parameters from the receiver type table (fixed here); regex::Error's message is regex-syntax's multi-line one ("regex parse error:" then the caret lines)
Needs: rebase and push of 145faff1 and this commit by the supervisor
Changelog: `std::regex`: `regex::dynamic(pattern)` compiles a run-time pattern to a `regex::Regex[regex::Dynamic]` (throws `regex::Error`) with `is_match`, `find`, `find_all`, `captures` (`group`, `named`), `replace_all`, `split`; a match is a `text::Match` with `m.text()`, a view of the searched text. A function taking a generic `std` type now lends it to that type's own methods.
Owner: none
Disk freed: none (no target/release/nikaia-scratch; project-tests cache 839M, new entries over 100 MB: two rlib 269M 265M, one run-bundle 286M; 24G free)
Rs2nika: none
Model: Opus 5.5
