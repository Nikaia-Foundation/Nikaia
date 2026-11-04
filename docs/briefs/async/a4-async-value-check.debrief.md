1. **What cost the most:** three full `tests/compiler --both-settings` runs, about 30 minutes each, where one would have done.
   - Runs 1 and 2 used scratch copies of the binary named `nikaia-new` and `nikaia-new2`. The fmt, trait-column, bounds and commit tests look for a parent process whose binary is named `nikaia`, then fall back to `target/fast/nikaia`. Those copies broke the lookup.
   - The fallback error ("no such file: target/fast/nikaia") hid the cause. At first I wrongly blamed it on the concurrent `it` build.
   - Before the break, the most turns went into the first NK1245 on a run lambda's type, and generating the `.stderr` files.
2. **What I had to look up:**
   - Part II's async examples, `docs/specification/10-nikaia-light.md`, especially #106: "a pending binding named in another task is waited for in that task". Only the regenerated `EXPECTED.txt` showed me that my NK2110 rule refused it.
   - The self-hosted badge procedure in builder-facts.
   - How `commit.nika` stages files: it cannot list a deletion that is already staged.
   - The brief could have named the spec blocks the change affects, and said that `EXPECTED.txt` must be regenerated after a parser change.
3. **Checks:**
   - Useful: clippy, pre-push, the lower-std fixpoint, the rerun of the 3 failing `it` tests, tests/compiler run 3, and running each new test against the old binary.
   - Wasted: tests/compiler runs 1 and 2, because of the binary's name.
   - Partly wasted: the whole `it` binary under load (30 minutes, 2 failures caused by the load). The brief only asked for the parser, async and spawn filters.
4. **Most useful change:** one line in builder.md: "Run `nikaia test` with `target/fast/nikaia` itself, never a renamed copy: tests find the toolchain by the parent's binary name `nikaia`. After a parser change, run `specification::` and regenerate `tests/specification/EXPECTED.txt`." Both cost me full runs.
