1. **Most cost:** the `it` run (about 3.5 min) and the two `lower-std` plus build cycles. Most turns went into finding the right forms: the signatures of `process::run`, `fs::Path`, `process::Output` and `sha256_hex` in `std.contracts`; how a host trait method that throws lowers (`Box<dyn Error>`); and that `CheckWall` had no `impl Error`. One `lower-std` round was lost to NK1102 (`String?` compared with `String`). `sysroot_holds` already did that check, so I reused it.
2. **Looked up, could have been in the brief:**
   - `process::run` takes `dir: ref String` and `Vec[String]` arguments, so paths go through `.display()`.
   - `Output` has `ok`, `stdout` and `stderr` as text.
   - `CheckWall` has no `impl Error`, and a thrown host error is boxed. That means the brief's "driver throwing CheckWall" needs the `RefCell` pattern; the brief should have said so.
   - `Thrown::split()` gets the error back on the Rust side; `message()` is `pub(crate)`.
   - `sysroot_holds` exists.
   - The path of the rs2nika binary.
   - `cargo fmt` uses edition 2024, so a plain `rustfmt` sorted the imports differently.
3. **Checks run:**
   - `nikaia check` on the single file: half wasted, since it cannot see `CheckWall` in another file and gave a false NK1151.
   - Old-binary `lower-std`, `cargo check` of `nikaia-std --features compiler`, then `cargo check`, build, `lower-std` fixpoint, the four `it` filters, clippy, `fmt`, `pre-push-check`, `badges.py`.
   - `--lib grammar_run::` was wasted: the module has no unit tests.
   - Not run: the whole `tests/compiler` package the brief asks for. The run rules forbid suite-sized runs; the driver covers it.
4. **One change that would save the most:** a *Forms* block in the brief. It would give the exact `std.contracts` signatures it relies on (`process::run`, `Output`, `fs::Path::display`/`parent`, `sha256_hex(ref Vec[u8])`) and the throwing-host pattern: keep the `RefCell`, rethrow a placeholder, return the kept value through `Thrown::split().0`. It would also say up front that `CheckWall` needs an `impl Error`.

After my commit, `grammar_run.nika` changed on disk, probably another session's edit. b070adb9 holds my version, and the driver should check whether the change is wanted.
