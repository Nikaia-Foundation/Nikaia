Result: done
Files: crates/rs2nika/convert/src/tr_items.nika crates/rs2nika/convert/tests/free_impl_outside.args crates/rs2nika/convert/tests/free_impl_outside.in/f.rs crates/rs2nika/convert/tests/free_impl_outside.stdout crates/rs2nika/convert/tests/corpus.in/manifest
Commit: rs2nika: a type the cut does not declare has its methods freed too
Body: frees_type(ty) replaces struct_here (false only for an enum the input declares), used by the impl writer and impl_has. Corpus: attributes 76/440, defaults 49/187, function 9/49, program 32/137 (floor 30 -> 32), seeds 53/115, all 219/928.
Pushed: local 9d974eb364240ac41b5641e491c032983f20651b
Tests: rs2nika nikaia test 145 passed (free_impl_tree unchanged); corpus run
Gate: green
Deviation: Display impl stays a todo comment (ADAPTER), as the converter treats Display; the test shows that.
Found: none
Needs: none
Changelog: rs2nika frees the methods of a type the cut does not declare
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
