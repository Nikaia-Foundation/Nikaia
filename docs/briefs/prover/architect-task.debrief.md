1. **Most cost:** finding out that the figure couldn't be trusted. Running `check --bounds` per file was quick, but it took several more steps to see that 227 of the 246 files are refused when checked alone, and to trace how `lower-std` checks the package (`sysroot.rs`). Tracing the loop defect also took several probes (inside the loop, after it, with and without `prove-through-loops`) before I reached `prover_state.nika`'s `through_loops`. That probing was worth it.
2. **What the brief could have handed me:**
   - Where the `tools` files are. `tools/*.nika` matched nothing; they are in `crates/nikaia-std/src/tools/`.
   - Where the certificate checker is: `lg_verify` and its callees in `logic_search.nika`.
   - That `tests/compiler` is a single project, so no test can set its own `--optimization`.
   - The exact headings of `builder-facts.md`. I first wrote an invalid `Facts:` line into the issues and had to fix them.
   - Which issues are already closed (#382, #383), and ADR-306 D12 (lengths below 2^60).
3. **Checks:**
   - Useful: the per-file count and its error codes; the shape probes (counter, `if` value, field, parameter, element); the loop-assert probes; running p3's probe exactly as written in the brief.
   - Wasted:
     - an over-broad `grep` whose output matched every path and had to be saved to a file;
     - `check` on a directory, which only says *Is a directory*;
     - the first probe, which failed on a syntax slip (`s: mut S`);
     - searching `tests/` for `prove-through-loops`, which found nothing.
4. **The change that would save the most:** a short *Lane facts* block in the architect task, or `state.md` once it exists. It would list the tools directory, the counting command and why counting each file alone undercounts, where the certificate checker is, the test-project limit, and the valid `Facts:` headings. With that, about 15 of my roughly 45 turns would not have been needed.
