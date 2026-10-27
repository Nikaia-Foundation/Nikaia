Result: done
Commits: f1746cd9 (Fixes #874), aeb02ace (Fixes #873), 4456f1fd (Fixes #872), all pushed fast-forward to develop
Tests: nikaia test --both-settings --project tests/compiler 141 passed; it subsets 299 passed; guard set 22 passed; clippy clean; pre-push-check clean; every new test failed on the unfixed compiler
Gate: green
Deviation: #873 reuses NK1201 (the code f-string holes already use for no text form); NK1302 and NK1303 are unused
Found: the Changelog line of f1746cd9 says "NK1260-adjacent `Eq`/`Ord` notes", sloppy wording; read it as the check_compare `Eq`/`Ord` notes. `char` is left only in code comments. The guide's Memory workaround (ff8d00d) can be undone now that #872 is fixed (not my file)
Needs: none
Changelog #874: NK2105, the `Eq`/`Ord` notes and the `[v; n]` note name `scalar`, not `char`; Part I likewise
Changelog #873: print/println/eprint/eprintln of a value with no text form is refused NK1201 before lowering, as `f"{x}"` already was; Part I 2.5, Part III NK1201 row
Changelog #872: a view handed to an owned parameter the body only reads is lent as it is (ADR-094 D1); a parameter the body keeps still gets NK1102, now saying it is a view and helping with `.clone()`; Part I 6.6
Disk: target/nikaia-project-tests 4.2G; / 18G free of 252G; new entry over 100 MB: run-bundle/65d964130cc06fb2 (293 MB)
