1. **What cost the most:** cutting the scratch files for 12 rows and converting each one (`cut.sh`, `share.sh`), then measuring the draft shares. The first P5a cut ended inside `impl Project` and had to be redone. Before compaction I also spent turns on `rs2nika --help`, which the binary read as a file name, so I took the flags from its source instead.
2. **What the brief could have handed me:**
   - the item and line map of the six files (I built it from `grep -n` on `fn`, `impl` and `struct`);
   - the converter's command line (`--profile check-free --prefix X_ --file`) and how to compute a share;
   - the Part III 17 gaps already mapped to #920 and #922;
   - the M2/K1 pattern for `Parsed` (host plus arena).

   All of these come back in every planning brief.
3. **Checks run:**
   - `scripts/pre-push-check.sh` passed. It prints nothing on success, so I ran it a second time just to see the exit code; that run was wasted.
   - The guard set passed (22 tests). For a docs-only commit it proves nothing, but it was cheap.
   - The disk check: 0 freed, cache 4.2G, 3.9G free.

   No clippy and no build. One correction to the report's `Gate: green`: it covers only those two checks.
4. **Not verified, please check before briefing from the section:**
   - "about 45% comments";
   - `exec` being #281;
   - the line counts in *Stays Rust*.

   These are estimates I wrote while drafting the section; I did not re-check them against the sources.
5. **One change that would save the most:** a checked-in `nikaia run scripts/plan_rows.nika -- FILE:RANGES PREFIX`. It would cut the ranges, wrap methods in their `impl`, run rs2nika and print the draft share and the TODO reasons. Every planning brief could then name it instead of each agent rebuilding it in the scratchpad.
