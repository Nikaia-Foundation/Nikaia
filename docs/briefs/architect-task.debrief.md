1. **Most turns:** finding out why CI was red. The job logs could not be fetched (the log URL returned 403), so I took the failing test from the check-run annotations instead. Then about 8 turns went on narrowing the formatter refusal: three probes that formatted fine, a loop that formatted the file item by item to find the first bad one, then two more probe rounds.
2. **Looked up that the input could have given me:**
   - `architect-input.txt` said only `BATCH DONE` and the `RED` file was empty, yet CI was red at 0f9a525. If the input had named the failing job and test, I would have saved about 4 calls.
   - That the formatter is self-hosted (`tools/fmt.nika`, called from `fmt.rs:21`).
   - Where `lower-std` runs its check (`sysroot.rs:281`).
   - The real matching rule for `[forms]` rows, which I read off P5b's rows because the header comment in `rs2nika.names` is wrong.
3. **Checks run:**
   - The runs list and failed jobs through `gh api`, and the annotations.
   - `nikaia fmt` on copies of the two new files, and the bisection loop.
   - `where.nika` for NK1125.
   - **Wasted:**
     - Fetching the job logs twice; both times only the 403 came back.
     - The `where.nika` call, which listed tests but no place where NK1125 is raised.
     - Two probes that looked for tuple-closure lines with `grep` before I bisected.
4. **The change that would save the most:** when CI is red, `drive.nika` should write the failing jobs and the annotation messages into `architect-input.txt`; today the line is `RED`, or `BATCH DONE` as in this run. Second: fix the header comment on `[forms]` in `rs2nika.names` (it is the converter lane's file) so it says rows match the Nikaia draft text.
