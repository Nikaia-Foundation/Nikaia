1. **Most cost:** a second build cycle. My first fix also let the unbound `text(d:P+)` pass its names to the guard. A probe after the build showed that form reaches rustc, because the emitter binds inner names only for a bound `text(..)`. I narrowed the fix to the `Bind` arm and built again. I also read the gate's output files once before the run had finished, which wasted a turn.
2. **What I looked up that the brief could have handed me:**
   - `text(..)` parses as `Pattern::Ref`, not `Pattern::Call` (the brief pointed at `grammar_call`).
   - The action's names come from `bindings_of`, a separate walk that takes every inner name, so the action and the guard follow different scope rules.
   - bdbe8a7a's rule that inner names are kept only for a *bound* `text(..)` outside a repeat, choice or built-in's argument. That was in the commit message, not the brief.
   - The Part II sentence is a row of the built-ins table, so the brief's grep (`text(` plus "bind") found nothing; I found it through `git show bdbe8a7a`.
3. **Checks run:**
   - Useful: the winnow rev trap, the probe on the old binary (NK1117), probes `p`/`q`/`r` after each build (`q` and `r` found the narrowing and the separate defect), one background gate (both-settings `tests/compiler`, `packages/yaml`, the guard set, clippy), and pre-push-check.
   - Wasted: the first build of the too-wide fix, and the early read of the gate's files.
4. **One change:** the brief's matrix of probes should list the unbound case (`text(d:P+) if(d…)` without `s:`) next to the bound one, and say which the emitter supports. Run on the old binary before the first build, that would have saved the second build cycle and found the separate defect at once.
