1. **What cost the most.** Designing the host interface. The brief named `parse_unit` but not how `check_imports` and `declared_once` would reach a `Parsed` that Rust holds by index. I read about 8 files to settle this (`project_members` hosts, `sysroot_cache`, `BtHost`, `ParsedView`, `fs.rs`, `block_on`). The second biggest cost was a full clippy + `it` round (about 8 minutes) lost because another session reset `lowered/` while I worked.

2. **What I looked up that the brief could have given me:**
   - `fs::read_to_string` is async, so the collect functions need `block_on` on the Rust side.
   - How to build an `fs::Path` from text (`let p: fs::Path = "."`).
   - Hosts that already existed: `ProjectMembersForeignHost` has `sha256_hex` and `read_beside`, but its Rust implementations are private to `project.rs`, which I was told not to touch.
   - `Noted` and `Checked` are Rust types, so the inference callback could not move.
   - The `Unreadable { what, why }` pattern that keeps "cannot read" byte for byte.
   - What `IoError` prints, to check the cause text no test reads.

3. **Checks I ran:**
   - Useful: the `lower-std` cycle with a fixpoint check, `tests/compiler` once with my copy of the binary, the `it` filters plus the guard set together with clippy, `pre-push-check`, and `rs2nika --score`.
   - Wasted: the first clippy + `it` run (on the reset tree), and `--lib modules::`, which matched 0 tests because the module has no unit tests.

4. **Mistakes in my report:**
   - Listing the tree reset under `Found:` was questionable: it is a process problem, not a code defect, yet it cuts the batch.
   - `Disk freed: 0` left out the binary copies I deleted from the scratchpad.

5. **The change that would have saved the most:** a builder.md rule to `git status` the `lowered/` files and the index (`lowered.rs`, which sits outside `lowered/`) right before tests. Second: a brief that states each host method's signature, not just its name, including how the per-file checks reach `Parsed`, so no reading is needed to design the interface.
