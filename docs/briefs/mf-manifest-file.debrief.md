1. **Most cost:** I ran the `it` test set twice, about 2 minutes plus a rebuild check. My grep pattern `test result` also matched the `result_source::` test names, and `head -40` cut off the summary. The second run went to a file but was grepped the same way, so I had to filter a third time. After that, the `tests/compiler` run (about 10 min) failed on `agent_script`, and it took two more turns to trace that to the machine (under 3 GB free disk), not my change.
2. **Looked up:**
   - How a Nikaia `BTreeMap`, `fs::Path?` and `i64?` field lower, and whether lowered structs derive `Default`. They don't, hence `manifest_empty`.
   - Whether `?? throw` and calls to throwing functions without `try` are accepted.
   - The `toml` version (0.9, which sets how `Display` writes a value).
   - Who builds a `Manifest {` directly (only `manifest.rs`).
   - That `rs2nika` sits at `crates/rs2nika/convert/target/nikaia/bin/rs2nika`.
   - That the MF plan row is at line 843.
   
   The brief could have stated all of these in one line each.
3. **Checks run:**
   - **Useful:** `lower-std` with the old binary (caught nothing, but cheap), `cargo check`, the build cycle, `--lib manifest::`, the `it` filters plus the guard set with clippy beside them, `pre-push-check`, the fixpoint `git status`.
   - **Partly wasted:** the second `it` run, which was my grep mistake. The `tests/compiler` run wasn't required by the gate (I touched no `check_*` or `emit_*` file), and its one failure came from the disk.
   - **Not run:** the `rs2nika --score` was cheap but uninformative, since the target shape was fixed by the brief.
4. **One change:** builder.md's test filter should be `grep -E '^test result:|FAILED|panicked'` (anchored), with output saved to a file. Also: check free disk (over 3 GB) before `tests/compiler`, since `agent_script` fails below that. Separately, a brief that redesigns the shape (TOML stays Rust, a flat item list into Nikaia) should not order a converter draft or apply the 60% rule. A 7% score here flags no converter defect.
