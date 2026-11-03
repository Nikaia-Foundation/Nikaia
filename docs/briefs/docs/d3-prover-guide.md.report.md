Result: done
Files: guide/prover.md guide/contracts.md guide/command-line.md guide/syntax.md guide/ownership.md guide/getting-started.md guide/grammars.md
Commit: guide: where the prover stops today, and what is planned
Body: Step 2 of the prover chapter (step 1: 5a4a17f7). Where it stops, each probed with `target/fast/nikaia check --asserts/--bounds`: a product of two names, an `if` as a value, floats, list elements (no quantifier), a value handed `mut`, text contents and `bool` calls (`is_sorted`), carry-back through a recursive function, a count-down index; under `warn` a project build checks the claim at run time and stops on `[3, 1, 2]`. Plans from ADR-381, ADR-361 D5/D6 (octagons, k-induction, PDR and `nikaia prove --propose` not built; guess-and-check, intervals, closed forms are), facts-about-data and the issues `--bounds` names; issue lines describe each issue from the repository (no GitHub access for titles). Linked from contracts.md and the other guide pages; command-line.md's "proves every `assert`" now reads "tries to prove".
Pushed: local d63ac12cb65273a6895fa16df17406b3a5a03937
Tests: every example run with target/fast/nikaia; relative links of prover.md resolve; pre-push green
Gate: green
Deviation: none
Found: (1) an index computed by a division keeps its check after its bound is proved: `fn mid(xs: Vec[int]) -> int { assert(xs.len() > 0); let h = xs.len() / 2; assert(h >= 0 && h < xs.len()); return xs[h] }` - both asserts proved, `check --bounds` says `xs[h] checked: no fact bounds what it reads`; `xs[xs.len() / 2]` under the same precondition says `a parameter no precondition bounds` (Part III 14.2 / 13.3 ADR-306). (2) Part III 14.2 states ADR-381 properties as built; `assert(is_sorted(xs))` is NK1300 "not a comparison of whole numbers", and `nikaia run --unproved-claims=warn FILE` still refuses it (a project build passes), #797 step 0
Needs: none
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
