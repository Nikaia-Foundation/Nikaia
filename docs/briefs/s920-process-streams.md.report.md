Result: done
Files: crates/nikaia-std/src/process.rs crates/nikaia-std/std.contracts docs/specification/30-nikaia-tooling.md docs/specification/adr/adr-243.md tests/compiler/tests/process_inherit.nika tests/compiler/tests/process_inherit.stdout tests/compiler/tests/process_inherit.stderr
Commit: std: process::run takes stdin, stdout and stderr options (Fixes #920)
Body: Steps 1-3 in one commit. New enums process::Stream { Captured, Inherited } and process::Input { Closed, Inherited, Text(String) } (process.rs, std.contracts), and options stdin/stdout/stderr appended after within; the defaults keep Stdio::null/piped/piped, so today's behaviour is byte for byte unchanged. An inherited stream is Stdio::inherit and leaves "" in Output; Text is written by a third thread while the pipes are read, a BrokenPipe on that write is ignored, other write errors are returned; the process-group kill of within does not touch the pipes. The option path needed no compiler change: the emitter passes options from the contract signature, as for within (2adf2c2c) and par::Order. Part III 17.2 paragraph after the deadline one, ADR-243 D10 amendment citing #920. The test's .stdout was taken from --bless after a typo in the hand-written one.
Fixes: #920
Tests: process_inherit and process_within with nikaia test --both-settings on a copied project (2 passed); cargo test -p nikaia --test it -- process (35 passed); cargo test -p nikaia-std --lib process:: (6 passed); clippy -D warnings local 1.97 (workspace) and 1.99.1 (-p nikaia-std) clean; cargo fmt; pre-push-check.sh green; lower-std changed no lowered file. The new test was not run on the old compiler (the options did not exist there).
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: std::process::run gains stdin: process::Input = Closed (Inherited, Text(String), written while the output is read, a child that stops reading is no failure), stdout: and stderr: process::Stream = Captured (Inherited goes to the caller's stream, "" in Output) (ADR-243 D10, #920)
Owner: none
Disk freed: 0 GB (target/nikaia-project-tests 3.4G, 7.7G free on /; new entry over 100 MB: run-bundle/3f8b93b88f7f213d 347 MB; /home/user/nikaia-clippy-new deleted)
Rs2nika: none
Pushed: local 8eb22790f8c00ab4af31090cddc553e92cf508ce
