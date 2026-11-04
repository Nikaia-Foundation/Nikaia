1. **Most turns and time.** Two full lower-std and build cycles of more than 10 minutes each. The second was needed only because the task-block shape (p3) takes a different path in `emit_function`: there the function body has no group of its own (`group == 0`), and I had set `async_pausing` only in the other branch. Tracing all three shapes through `emit_function` before the first build would have saved that cycle. Before the context summary, most turns went into reading `task.rs` and `group.rs` and designing the held-handle runtime. That included finding out that `PausedScope`'s invariant `'scope` rules out borrowing the body's locals.
2. **Lookups the brief could have handed me:**
   - The issue's probe does not compile. It needs `use std::fs` (not `import`), the two-argument `read_to_string(p, fs::Root::Anywhere)` and a `catch`.
   - NK2110 refuses a binding read in the very next statement, so each probe needs a statement in between.
   - `nikaia test` has no `--filter`, although the brief says it does. I had to build a scratch project of the async, group_scope, overlap and `task::` tests, and `parser_grammar_words` fails there because it reads a repository path.
   - There are no `task_*` tests, so that half of *Done when* is empty.
   - Where the walk sets `in_lambda`, and that `emit_async_binding` runs with `flow.statement == -1`, which is the cause of the -1 refusal.
3. **Checks I ran:**
   - `cargo check` of std.
   - The cycle twice, with an md5 fixpoint check.
   - The probes and the new tests on both binaries.
   - The scratch suite with both settings (useful: 45 of 47 pass).
   - The single-test project for the `.stderr`, which caught a missing trailing blank line.
   - `nikaia-std --lib task::`, clippy, and pre-push. Pre-push was red only for the badge, which was known in advance.

   Wasted: one probe run with the wrong paths (`p1.nika` instead of `p1/main.nika`), one try to `run` a probe directory as a project, and checking the first cycle's `git status` without a fixpoint comparison, which forced an extra rebuild just to find out.
4. **The change that saves the most.** Give the brief a *Probes* block with programs that compile, and a *Run tests* recipe that works: a scratch project made of the named test files, since `nikaia test` has no filter. Also name the two body branches of `emit_function` (the body as a group, and `group == 0` with an `async { }` inside). Separately, the step-2 span fix landed in the step-1 commit, so commit 1a410df3 holds only the test. The brief could say that steps sharing one function may be committed together.
