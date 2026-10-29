1. **Most turns:** finding the cause of the red CI. The CI logs could not be downloaded (the storage link returned Forbidden), so I worked it out from the b943 report and by running the test myself. The CI input also gave no failing test name. That cost about 6 turns, and running a single test file cost one more: `nikaia run` refuses a file inside the project, so I copied it to the scratchpad.
2. **Things I looked up that could have been handed over:**
   - The failing test names for a RED (here the cut line named it).
   - That CI logs are not reachable headless.
   - How to run one `tests/compiler` test.
   - The issues' probes as they stand today (#933's probe no longer passed the checker).
   - The NK2605 raise chain (`check_crossings` to `check_compare`) and the result-source code (`rsrc_*` in `views.nika`), found with about 5 greps.
3. **Checks:** the CI run list, the job steps, the byte dump of `.stdout`, running both probes, and three passes reading the checker code were all useful. Wasted:
   - two CI log downloads;
   - `where.nika -- NK2605`, which gave only test files, not where it is raised;
   - the `fs::open` probe, which I wrote with the wrong arity;
   - the first #933 probe, which ran into NK1102.
4. **The change that would save most:** when `drive.nika` stops on a red CI, it should write the failing job and test names, with the assertion lines, into `architect-input.txt`. It could take them from the test report the CI uploads as a file. Second: `where.nika NKxxxx` should list the places that raise the code in `tools/*.nika`, not only the tests that cover it.
