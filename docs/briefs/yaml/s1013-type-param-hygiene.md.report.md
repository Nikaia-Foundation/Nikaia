# s1013: a user type named S or E (main session)

Result: done
Pushed: 627af41e
Tests: new grammar_a_type_named_s_or_e_is_the_users passes (on the old revision rustc refused it: found type parameter `S`); tests/compiler 415 passed (the one failure was a binary built before 637bec45, passes rebuilt); packages/yaml 10 passed; clippy clean; pre-push-check ok
Deviation: none
Found: none
Changelog: A user type named `S` or `E` in a grammar rule's type or action is the user's: winnow-grammar moves to 9e23474f, whose generated type parameters are `__WgS`, `__WgEr` and `__WgB` (#1013).
Owner: none
