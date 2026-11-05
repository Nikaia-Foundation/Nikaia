Result: done
Cause: 4919b0bf (a `-` that begins a line begins an expression, ADR-390): `cargo test -p nikaia --test it fmt::every_nika_file_formats_to_the_same_tree_and_a_fixed_point`, benches/lowering/source.nika (a long `a + b - c - d` line) was cut before the `-`, so the formatted file parsed as other statements ("would change what the file means")
Fixed: tools/fmt.nika `fmt_operators` cuts after a `-` instead of before it (lowered/fmt.rs by lower-std); a `+` still leads its line
Tests: fmt:: 18 ok in the fast profile, including the failing guard test over the repository; a probe of a long `+`/`-` chain formats, runs the guard and is a fixed point; clippy not run (only lowered output changed); fmt, pre-push green
Found: none (note: a chain mixing `+` and `-` now has `+` leading and `-` trailing lines; the guard test over benches/lowering/source.nika covers it, no new Rust test)
Files: crates/nikaia-std/src/tools/fmt.nika crates/nikaia-std/src/tools/lowered/fmt.rs
Commit: fmt: a subtraction is broken after its `-` (the fix for the red of 4919b0bf)
Body: With ADR-390 a `-` that begins a line begins an expression, so the printer's cut before a binary `-` changed the tree and the formatter's guard refused benches/lowering/source.nika. `fmt_operators` now cuts after a `-`.
Changelog: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local bf5c0619809a12d82bab17d239bcb5271349ce14
