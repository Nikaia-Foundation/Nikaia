1. **What cost the most:** checking the earlier run's 129-file partial patch hunk by hunk, then debugging why the detached fallback wasn't bound. The cause was that `emit_stmt_block_plain`'s discarded one-statement trial render used up `walk.async_bound`. After that, three rounds of build and test went on NK1134 ("nothing can fail" on fallbacks with `within`) and on rustc "unnecessary braces" warnings in the bound wrappers. Each of these surfaced only after a full `lower-std` and build, not from one probe up front.
2. **What I had to look up:**
   - That a handle's `join` contract throws only `task::Crashed`, so a test can't match `TimedOut` by type at the join.
   - That `async` in a `fn main() throws` hits the #988 refusal.
   - That the checker needs a may-fail entry (`flow.guarded` plus the NK2605 path) for a deadline; the brief named the emitter entry points but not this checker hook.
   - That a `println(...)` followed by `-1` on the next line parses as subtraction.
   
   One line each in the brief or `builder-facts.md` would have saved those probes.
3. **Checks run:**
   - `cargo check`, then the `lower-std` and build cycle with its fixpoint.
   - `cargo fmt`, clippy, and `pre-push.nika`.
   - The `nikaia-std` `task::` unit tests and the `it` parser `async` tests.
   - tests/compiler `async_ task_ spawn_ overlap_` with both settings: 70 passed.
   - Probes per form, including the variable-deadline probe.
   
   **Wasted:** the first two runs of that last probe. My probe tripped #988 and then the `join` throws rule, so neither run tested anything new.
   
   **Caveat on the report:** it says `Gate: green` on the strength of these cheap checks. I did not run `gate.nika` or the gate in `.claude/agents/builder.md`.
4. **The one change:** builder.md should say that a file already removed with `git rm` makes `commit.nika` fail (`git add` refuses the path). List it under `Files:` but leave it unstaged and deleted in the working tree. Also, the brief should list the checker hook to add next to the emitter entry points, and the known refusals new tests must avoid: #988, and `join` throwing only `Crashed`.
