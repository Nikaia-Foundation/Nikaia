1. **Most turns:** finding out what works today. Each grammar feature had to be tried as its own small program: `recover`, `span`, values and guards, own `WS`, `members`, `comptime`, left recursion. Several probes failed for reasons that had nothing to do with grammars: a list in an `f"…"` hole, `u8::from_str_radix` whose type couldn't be inferred, `catch` handing back an owned `String` where a view was expected. Others hit real defects (`dec` repeated inside a lowercase rule, a commit point inside `recover`, #1001) and I had to narrow them down before I could write around them in the text. That was about 15 probe runs, most of them quick.
2. **What the brief could have handed me:**
   - The list of open grammar defects in `builder-facts.md` (#1001-#1004), which I found by grep.
   - That `span`/`here` over a `String` wait for #743.
   - That a parse's position values (`text::Span`) need `use std::text`.
   - The GitHub repo URL for issue links. I guessed it wrong first.
   - That the specification says nothing about left recursion.
   - Which examples use `@frame` and `dsl`.
   - That `command-line.md` has no *Where next* list.
3. **Checks run:**
   - **Run:** the probes; a scratch script that pulls each block out of the page, runs it and diffs it against the pasted output (run twice); a link and anchor check; `pre-push` twice; the calc example run for its outputs.
   - **Partly wasted:** the first runs of the block script, which also compiled the calculator pieces and reported them as failures I already expected. Also two probe rounds fixing mistakes in my own probes.
   - **Skipped:** cargo build, fmt, clippy and tests, since the change is docs only.
4. **The one change that would have saved the most:** give docs briefs a short *works today / not yet* table for the features the chapter must cover, with the issue numbers and the repo URL. Also ship a standard helper that extracts and runs a page's `nika` blocks and diffs their output (a `tools/*.nika` or `scripts/doc_blocks.nika`), so each docs agent doesn't rewrite one.
