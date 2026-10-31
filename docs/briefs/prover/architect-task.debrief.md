1. **What cost the most.** Sorting the 708 catch-all sites and the 439 counter sites by what blocks them. `proved.nika` gives a reason but not the form behind it, so I wrote two throwaway Python classifiers and ran about eight probe programs to compare the same loop in a one-file program with the same loop in the tools. That comparison found both causes behind p12 (unsized lengths, and structs declared in another file).

2. **What I had to look up that the brief could have given me:**
   - the report's line format and how to map sites to files (`state.md` had an awk line, but not the column layout);
   - where the length axioms live (`bounding_length_named`), and where `ensures_each` is parsed and read (`ty.nika`, `ledger.nika`, `bm_each`, `bounding_each`);
   - that the ledger keys `split` as `str::` but `chars` and `bytes` as `String::`;
   - the highest ADR number.

   `where.nika` found nothing for field names (`std_lengths`), so I fell back to Grep.

3. **Checks.** Useful: re-reading the suite log (green, which cleared the stale `RED`), one fresh figure run (45 s), re-running p9's Found probe, and the probes for #388's parameter case and for the i64 type bound. Wasted: the first `where.nika` call, an `emit_misc` line that had shifted, and one misleading classifier column ("let literal").

4. **One change that would have saved the most:** a tool that sorts each unproved site by the form that blocks it (how the indexed list was built, whether the counter sits in a `for` or a `while`, guard present, element type known or unknown), with an example per class. Build it as `proved.nika --by-form` or as the catch-all re-split, item 15. That classification is what every architect run in this lane redoes.

**Not finished:** commit 232e502 (ADR-385, the p12 and p13 briefs, `state.md`, `batch.txt`, `formal-verification.md` §5) is local only. I did not push it, and I wrote no report to `architect-task.md.report.md`. Whoever runs next should run `git push origin develop` and write that report: `Result: done`, `Batch: p12, p13`, `Figure:` 1,576/2,677 indexes and 846/1,633 operations at `prove-through-loops:on`, `Owner: none`.
