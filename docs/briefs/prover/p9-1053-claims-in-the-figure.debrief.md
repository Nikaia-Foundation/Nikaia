1. **What cost the most:** the pre-flight. The hashes 788981d and 76c216e had been rebased, so the brief's `git log | grep` found nothing. It took 3 turns to match them by subject (6816f47, ce758cb). After that, the scratch sysroot for the `assert` check took 3 tries. I didn't know its layout (`<root>/nikaia-std/src/tools`, plus `described.contracts`), and I wrote `assert x` where the syntax is `assert(x)`.
2. **What I looked up that the brief could have handed me:**
   - the `asserts in` header format (`check_facts.nika` :805);
   - that `check_against` always refuses unproved claims, so `refuses` is true and an unproved `assert` gets the whole file refused;
   - the text-method names Nikaia has (`matches`, `split_once`);
   - the sysroot layout that `tools_dir` expects.

   The brief's "13 panics in 9 files" had no counting rule behind it, so I had to work out why I got 3 in 3.
3. **Checks run:**
   - the build, once, plus `nikaia check` on the script;
   - `lower-std --bounds --asserts` at both settings, and `--bounds` alone;
   - the scratch-sysroot claim checks and a synthetic header line;
   - the named unit tests, fmt, clippy 1.97 and 1.99.1, `pre-push-check.sh`.

   Wasted: the first `lower-std --bounds` run without `--asserts`, which a synthetic report could have covered. Also two sysroot runs that failed on layout and syntax. The `git rev-parse` probe was not wasted: it found the temporary-view defect.
4. **One change:** have the pre-flight match commits by subject (`git log --grep`), not by short hash, since the driver's rebases change the hashes. Second choice: a figure the brief quotes, such as the panic count, should come with the command that produced it.
