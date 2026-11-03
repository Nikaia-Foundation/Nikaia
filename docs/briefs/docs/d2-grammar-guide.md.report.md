Result: done
Files: guide/grammars.md guide/syntax.md guide/contracts.md guide/ownership.md guide/getting-started.md
Commit: guide: grammars chapter, part two: building a real parser
Body: Part two walks through examples/calc step by step (tokens, precedence as rule order, tails folded left, recursion through brackets, naming a rule for messages), then commit points, actions that throw, an own WS with comments, rules with values and guards, recover, span over Bytes, and a grammar called in a comptime binding, each as a program run with target/fast/nikaia and its output pasted. Left recursion is described as not specified (the calculator keeps head-and-tails); span/here over text is named as waiting for #743; the group of differing yields behind the BLANK rule as #1001; recover's message not reaching the program and a cut inside the recovered rule ending the parse are named as not available yet. syntax.md's Grammars section gains a short runnable grammar and links the chapter; the Where next lists of syntax, contracts, ownership and getting-started link it (command-line.md has no such list). Part one is d9327d37.
Pushed: local 1aefb088518d961ef8c491048f09c8f35cd1d716
Tests: every complete program in guide/grammars.md and the new block in guide/syntax.md run with target/fast/nikaia and diffed against the pasted output (12 + 1 match); the five calculator fragments are verbatim from examples/calc, which was run for the shown outputs; relative links and anchors checked by script; pre-push green; disk after: target/nikaia-project-tests 2.7G, / 7.9G free, no new cache entry over 100 MB
Gate: green
Deviation: none
Found: (1) a grammar entry called without catch in a function without throws reaches rustc ("The `?` operator can only be used in a function that returns Result") instead of NK2605; probe: `grammar P { entry rule n -> i64 = x:dec[i64](digit+) -> { x } }` and `fn main() { let v = P::n("1"); println(f"{v}") }`. (2) a cut inside a recover(p, sync) pattern ends the whole parse: `entry rule block -> Vec[i64?] = "{" => xs:recover(stmt, ";")* "}" -> { xs }` with `rule stmt -> i64 = "let" => n:dec[i64](digit+) ";" -> { n }` on "{ let 1; let x; let 3; }" fails at x (without the inner => it gives 1, null, 3); the recovered failure's message also never reaches the program (10.6/10.8 say "p's value or the report"). (3) dec[T](digit+) written directly in a lowercase rule and repeated fails at the end of the repetition instead of stopping it: `entry rule i -> Vec[i64] = xs:dec[i64](digit+)* eof -> { xs }` on "1 2" errors "expected a digit" (the same inside an uppercase rule gives 2 items); an overflowing dec reports "expected a digit" at the next character rather than that the number does not fit. (4) left recursion works today (`expr = a:expr "-" b:NUM | n:NUM` gives 10-3-2 = 5) but Part II does not specify it, and examples/calc's comment says it would never terminate.
Needs: none
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
