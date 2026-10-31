1. **Most time:** the two probe runs in a scratch copy of the package, a few minutes each. The first run was wasted by my own mistake: I assumed `yaml::parse` returns one `Node`, but it returns `Vec[Node]`.
2. **What I looked up:**
   - The `pub fn` signatures and tree types at the top of `packages/yaml/src/main.nika`.
   - How each grammar rule fills `Scalar.raw`.
   - #928's body, fetched through `gh`.
   - The rules block, copied out of y21.

   A list of the package's public signatures in the task brief would have saved the first lookup and the wasted run.
3. **Checks:**
   - The y21 probe: needed, because it confirmed the gap and the tag-start span (28) before I wrote the decision.
   - `pre-push-check.sh`: cheap and fine.
   - `git ls-files` before `git mv`: needed.
   - Wasted: the first probe run.

   On the way I noticed that a field read on a `Vec`, `n.value`, gets a misleading NK1245 ("can't tell what type"). I did not file it. It probably deserves an issue: the message should say that `Vec` has no field `value`.
4. **The change that would save the most:** a script, e.g. `scripts/yaml-probe.nika -- <file.nika>`, that copies the package to the scratchpad, drops the test in and runs `nikaia test --project`. The architect's probing before each brief, and the builders' gap probes, would then each take one call instead of hand-written shell. Next to it, the task brief's *Lane facts* should list the public API: `documents`/`parse -> Vec[Node]`, `text`, `as_i64`, `as_f64`.
