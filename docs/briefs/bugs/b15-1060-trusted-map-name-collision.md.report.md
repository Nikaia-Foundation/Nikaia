Result: done
Files: scripts/gate.nika tests/compiler/tests/gate_full_dry_run.stdout
Commit: gate: a lowered file that drifts from what this compiler lowers is red (Part #1060)
Body: After the first lower-std of the lower-std step, `gate_lowered_drift` lists the lowered files `git diff HEAD` shows rewritten whose `tools/*.nika` is neither changed by the commits under test (base...HEAD and --files) nor edited or new in the working tree, and the index `lowered.rs` when no such `.nika` exists at all; any of them makes the step `red lowered drift: <files>`. Before this, the gate's fixpoint ran on the rewritten file and came out green while CI, checking the committed file, was red (b070adb9). gate_full_dry_run's .stdout carries the new dry-run wording of the step. The concatenation is written `edited.stdout + "\n" + added.stdout` over the two `process::run` results kept whole, the form gate_run uses, because `let a = process::run(..).stdout` then `a + "\n"` reaches rustc (see Found).
Part: #1060
Tests: probe (step 1) not reproduced on HEAD 6508f966: grammar_run.nika from b070adb9 applied, `NIKAIA_LOWER_CACHE_SALT` fresh, lower-std with the HEAD binary, with a binary built with grammar_run.nika in it, and with the 17:31 binary: project_members.rs stays as committed (TrustedMap) every time, applied files removed by name; gate_lowered_drift on a scratch repo, six cases (clean, a drifted .rs, its .nika in changed, its .nika edited, drifted index, index with a new .nika) all as meant; gate_full_dry_run passes through nikaia test --both-settings (own project); pre-push-check.sh green; no Rust or tools/*.nika changed, so no clippy, fmt, lower-std
Gate: green
Deviation: step 2 not built: the probe does not reproduce on HEAD, and the trace does not hold: `env.sources` is filled from the std ledger only (`std_ledger()`, emit/mod.rs:816), never from a tools file's host trait, and it is matched only against method-call names (`tm_deps_of`, trust.nika:896), never a variable, so a parameter named `manifest` cannot read as a source; the test the brief asks for would pass on the unfixed compiler. The HashMap in project_members.rs came from a stale binary (316e0721 committed it, 84fbfb99 restored TrustedMap), which is what step 3 now catches
Found: `let a = process::run("echo", ["a"]).stdout` then `let both = a + "\n"` passes check but rustc refuses it (`&String: nikaia_std::concat::Plus<_>` not satisfied): the let lowers as `let a = &process::run(..).await?.stdout;`, a borrow of a temporary's field, where it should move the field out
Needs: file the Found defect (bug lane); GR (grammar_run) may re-land: with it applied, lower-std on HEAD leaves project_members.rs unchanged
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: none
Pushed: local 1d88667d58f05c9dce9a63b8563ad3219d2dd2e5
