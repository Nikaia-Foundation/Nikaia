Result: done
Files: website/easy/async-without-colouring.html website/safe/conflicting-actions.html website/index.html website/easy/index.html website/safe/index.html website/safe/unhandled-errors.html
Commit: website: teach `async` instead of `overlap` and `spawn`
Body: async-without-colouring.html now leads with a runnable `async` program (two counts beside each other, the error thrown where the program waits), then describes calls without colouring, what is still being built (pausing tasks, handles, options, par_iter, the pool) and what the compiler checks. The home, Easy and Safe tiles show a built `async` snippet instead of the db::load example; the Conflicting actions tiles show the `async` group the specification refuses, with the chip now Partly checked and a note that this check between `async` tasks is still being built (#988), since the compiler accepts it today; the page drops its `overlap` program and diagnostic and keeps one sentence that older `overlap` programs get the check. The error passages say a task's error arrives where its value is first needed or at its group's end. good/ and fast/ use "overlap" only as an English verb and are unchanged.
Part: #983
Tests: ran with target/fast/nikaia every runnable block: guide/syntax.md (the async program, the failure program, the SharedMut group), guide/ownership.md (the borrowing task), website async-without-colouring.html's program, extracted from the files, all give the output shown; pre-push green
Gate: green
Deviation: none
Found: (1) an `async` task whose call pauses does not build: `let a = async size_of("a.txt")` with size_of calling fs::read_to_string panics in render_refusal (Span::new PosOverflow, the refusal at -1 a5b reported), and `add(async size_of("a.txt"), async size_of("b.txt"))` is refused "This lambda calls size_of, which can pause, but the compiler knows nothing about what the lambda is passed to"; ADR-async-5 D20 says a pausing task is polled by its group. (2) two `async fn { total += 1 }` / `names.push(..)` tasks in one group, and two tasks both calling println, are accepted (NK2107/NK2104 between async tasks not built; the a4 brief named it a later brief). (3) NK2101's note still says "A task started with `spawn`" for `async(detached)`, which already compiles (async_parse_detached_runs). (4) `async` inside an f-string interpolation is refused as not built (#988).
Needs: none
Changelog: The guide, README and website teach `async`: `async e`, `f(async g(), async h())` and `async { … }` groups replace `overlap` and `spawn` in every example, and what is still being built (handles, detached tasks, options, `par_iter`, pausing tasks, the conflict check between async tasks) is described with its issue
Owner: none
Disk freed: 0 (test cache 2.9G, 5.5G free; new entries over 100 MB: rlib 678 and 268, run-bundle 289, build-time-parsers 877 MB)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 3f22438bc106ba497fd2911a09bd57a5fd4d9d7e
