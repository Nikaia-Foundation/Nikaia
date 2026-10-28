1. **What cost the most:** changing `Settings` into `{ build, words: ProjectWords }`. It took about 6 `cargo check` rounds, because 42 field reads were spread over `project.rs`, `main.rs` and an `it` test. A small script put `words.` in at each line and column the compiler named. My first run of that script used a wrong scratchpad path and wasted a turn. Writing the Nikaia file took one go, and `lower-std` accepted it the first time.
2. **What I looked up that the brief could have given me:**
   - The callers outside `project.rs`: the `it` tests that call `optimizations` and `skip_free_at_exit`, and `main.rs`'s field reads.
   - What the lowered signatures look like (`throws` becomes `Result<_, Thrown<E>>`, `String?` becomes `&Option<String>`), and the `Thrown::split()` adapter from P4a.
   - That nothing in `tools/` uses `filter_map` yet (it lowered fine anyway).
   - How the plan marks a row as built.
   - Whether `ProjectWords` means a field inside `Settings` or only a type passed to the functions. The brief named it but left this open, and I picked the field.
3. **Checks I ran:** `cargo check` (several rounds), one `lower-std` / build cycle, lib `project::`, the brief's `it` filters plus the guard set, the three touched `it` test groups, `nikaia test --project tests/compiler`, clippy, `pre-push-check` (twice: once printed nothing, once for its exit code), and `rs2nika --score`. Probably wasted: `tests/compiler` (no `check_`/`emit_` file changed, and the brief asked for it anyway), and the first `pre-push-check`, whose output I cut away with `tail`.
4. **One change that would have saved the most:** have the brief list every file that reads or writes the struct being reshaped, with a count, and say whether the struct is split (`Settings.words`) or left as it is. That would have meant one edit pass instead of a guess and a loop of compiler errors.

*Open:* `project_settings.nika` changed on disk after my commit, and not by me. I did not check what changed. The supervisor should compare it with `aab4d749` before the next `lower-std`.
