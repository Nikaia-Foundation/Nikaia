Result: done
Files: crates/nikaia-std/src/either_text.rs tests/compiler/tests/either_text_into_path.nika tests/compiler/tests/either_text_into_path.stdout
Commit: std: text holding both kinds converts into fs::Path (Fixes #1106)
Body: Adds `impl From<EitherText<'_>> for PathBuf`: owned text moves in, a view is copied.
Fixes: #1106
Tests: new test fails on unfixed binary (rustc: PathBuf: From<EitherText> unsatisfied), passes with --both-settings in a one-test project; the wider either|mixed_arms|path set was not run (nikaia test has no filter)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: Text holding both kinds (view and owned) now converts into fs::Path (#1106).
Owner: none
Disk freed: 0 GB
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 1a5fdf6d6e16150649400d9f1183a199264818c0
