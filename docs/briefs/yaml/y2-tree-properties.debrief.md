1. **What cost the most:** the step 2 defect.
   - The checker's first message pointed at a parameter type (`String?` against `ref String?`).
   - I took four probe runs to isolate the real cause: a view field read from a struct bound in a grammar rule is refused when it goes into the returned struct.
   - Before that, a tab-count mismatch in my first edit script cost one turn.
   - Step 1 itself checked, ran and passed on the first build.
2. **What I looked up that the brief could have given me:**
   - The syntax for several named options in a rule call (`Block(n; c: c, k: x)`). I read it in Part II 10.
   - That the shipped `target/fast/nikaia` was older than develop, so I built once.
   - That `nikaia test --bless` plus `nikaia fmt FILE` is the whole loop for a package test.
   - Most of all, that the brief's own plan (properties parsed into a struct and copied onto the node) runs into a checker gap nobody had probed.
3. **Checks:**
   - Useful: `nikaia check` on `main.nika` after each edit (seconds), a scratch project running the new test, one package test run with `--bless`, `fmt::every_nika_file`, `pre-push-check`, and three small probes that isolated the defect.
   - Wasted: the first probe hit a name clash (`S` read as a type parameter), and the `match`-destructure probe was only for the report.
   - Not needed: `clippy`, since no `crates/` file changed.
4. **One change that would have saved the most:** for every new tree shape a step needs, have the architect run a two-rule probe before writing the brief: a struct built in one rule, its view fields copied into another struct. Here that probe would have shown the NK1106 defect first, so the compiler fix could have been scheduled ahead of y2 step 2. Second: on a rejected push, builder.md should say whether the builder may `git pull --rebase` a clean tree. CLAUDE.md asks for a rebase; the brief forbids it.
