1. **Most cost:** the background waits for the two clippy runs and the 24-test selection, not the code. Of the turns, two `lower-std` rounds were spent on Nikaia text rules: NK1214 refused cutting a text at `len() - 6`, and NK1185 refused `?? stem` because the two sides of `??` differed (`ref String` against `String`). I also spent one turn learning that `nikaia run` of the test program fails, because its `target/fast/nikaia` fallback is not found under `run`.
2. **Looked up that the brief could have handed me:**
   - The brief says a stray `.bounds` is an error "as a stray `.stderr` is", but `output_tests` silently ignores a lone `.stderr`. I had to choose where the error lives: a Rust-side check in `project::program_tests`, chosen to avoid the Nikaia throw-type question.
   - With a dependency, `check --bounds` prints a `--- path` block for every unit, so the entry's report has to be cut out of the output.
   - Failed program tests print through `Option<&OutputTest>` = `None`, so the explanation of a difference needed its own hook.
   - Text cutting in `tools/*.nika` takes only positions from `find`, and `strip_suffix(..) ?? ""` is the idiom to use.
3. **Checks run:**
   - Build and `lower-std`, which ran twice: before the build and as the fixpoint.
   - `cargo check`, clippy 1.97 and 1.99.1, the `project::` unit tests, `pre-push-check`.
   - The scratch project: bless, re-run, and the old binary against it.
   - The 24 selected tests, with `--both-settings`.

   Wasted: running `check --bounds` from inside `tests/compiler/tests` (it read the whole project and printed parse errors from other files), and the `nikaia run` attempt on the test program.
4. **One change:** add to builder-facts: "to run some `tests/compiler` tests by name, copy `nikaia.toml`, `src/` and the selected `tests/NAME.*` into a scratch project and run `nikaia test --project` there; never `check` a file inside `tests/compiler`." Also have the brief state that `check --bounds` with dependencies prints a block per unit.
