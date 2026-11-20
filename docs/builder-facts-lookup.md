# Builder facts: lookup

The traps already met and the rules for moving Rust into Nikaia; the short
*Core* every builder gets is in `docs/builder-facts.md`. Nobody reads this file
whole: `nikaia run scripts/syntax.nika -- KEYWORD` prints the lines of a topic
(with the spec section and an example test), and `scripts/agent.nika` puts the
sections a brief names (`Facts: Probes, Grammar`) into the prompt, or, without
that line, the sections whose `Paths:` line (the first under each `###`
heading: path fragments and words, any case) has a fragment in the brief or its
issues, at most three (#2002). A new trap goes into the section of its topic,
in one line, with the run that paid for it; a new section gets a `Paths:` line.

## Traps already met

### Parser nodes that change

Paths: `parser`, `Expr::`, `AsyncGroup`, `syntax tree`

* **`Expr::AsyncGroup` gains two `Option` fields** with #986 (the async
  lane): a new match on it in `tools/*.nika` is written with
  `..`, so it does not break when the fields land.

### Tests and the gate

Paths: `tests/compiler/tests/`, `.stdout`, `.stderr`, `--bless`, `nikaia test`, `cargo test`, `fails-before`

* **One program alone**: `nikaia test --project tests/compiler --filter NAME`
  (several `--filter`s allowed, #1238). Without `--filter` it runs everything:
  write the output to a file and grep it, never `| tail` (b933).
* **`--bless` creates a missing `.stdout`**, never a `.stderr`; `--bless-stderr`
  does both. A program the compiler refuses is blessed into its `.stderr`
  only (no `.stdout`) when that `.stderr` exists or `--bless-stderr` is given;
  with neither, plain `--bless` fails it and writes nothing, so a test meant to
  run that fails so is a refusal to fix, not a missing flag
  (`project_tests_bless_program`, #1238, #1236; issue-1431). With `--filter` it blesses only that test. A new test is run
  through `nikaia test`, not only `nikaia run`: `print` writes no newline and
  the `.stdout` holds the bytes exactly (7c1643d).
* **Only `.stdout` holds raw bytes**: `nikaia test` reads a program's
  standard error lossily as UTF-8 (`project_tests_relocated`, #1182), so a
  `.stderr` or `.stops` test cannot show non-UTF-8 bytes; test those on
  standard output and keep standard error ASCII (issue-1859, #1860).
* **`tests/compiler` runs `target/fast/nikaia`**: never run the `it` binary
  beside it, it relinks the binary (j2). Some tests spawn `target/fast/nikaia`
  by relative path and fail from a copy (`a_trait_column_*`,
  `a_library_trait_with_no_impl_runs`, `bounds_expectation_guards_a_proof`;
  v16): run those with `target/fast/nikaia`. A failure from a vanished binary
  is not yours: re-run that test singly. An untracked test of another agent
  may fail: name it under `Tests:`.
* **`--both-settings` runs past 10 minutes**: background with the long timeout (y9).
* **`cargo test ... --test it -- a:: b::`**: the `--` is needed. Read an `it`
  run with `grep -E '^test result:|FAILED|panicked'`, anchored (M3b).
* **A parser change runs the whole `it` binary**, no filter: many tests hold
  programs as Rust strings (9936c70).
* **A change to what the lowering writes**: `grep -rn '<old fragment>'
  crates/nikaia/tests/it/` and run each test it names (b1029).
* **A change to inference or to a bare `self`** moves baselines outside
  `tests/compiler`: the specification's `EXPECTED.txt` (regenerate with `cargo
  run --profile fast -p nikaia --example specification >
  tests/specification/EXPECTED.txt`), `COMPILES.txt`, and the examples'
  ledgers. Look at `git status` before the commit (v11).
* **A program that stops at run time is a test with a `NAME.stops`** (#1260,
  ADR-269 D22): the file holds what the stop writes on standard error, and the
  test passes only when the run stops (exit 101) writing exactly that
  (`NAME.stdout` compared too). `--bless` rewrites a `.stops` but never makes
  one: create it empty, then `--bless --filter NAME` fills it.
* **A use after a move is refused only by rustc today** (#1241): `nikaia test`
  counts it as a failed build, so it cannot be a `.stderr` test yet.
* **Guard tests scan the whole repository** (`range_spelling`,
  `shipped_ledger`, the std ledger in `contracts`): a script's `a..b` or a new
  `unsafe fn` fails them.
* **Clippy here is not clippy in CI**: CI takes the newest stable. When `rustup
  check` shows a newer one and your change touches Rust, also run
  `CARGO_TARGET_DIR=/home/user/nikaia-clippy-new cargo +<version> clippy
  --all-targets -- -D warnings` (install with `rustup toolchain install
  <version> --profile minimal -c clippy`), then delete that directory; never
  `rustup update stable`.
* **CI has no `target/fast`**: a test that runs the toolchain takes the
  `nikaia` that runs it (`readlink /proc/$(cut -d' ' -f4 /proc/$PPID/stat)/exe`).
  A red test that passes locally: check what CI lacks first.
* **Each `crates/unsafe/*` crate is its own workspace**: it has no `fast`
  profile (`cargo test --profile fast` there fails, *profile `fast` is not
  defined*), and a plain `cargo test` inside it builds a cold `target/` of
  its own. Run its checks from the root against one shared target dir, kept
  between runs under the git-ignored `target/`: `cargo test --manifest-path
  crates/unsafe/NAME/Cargo.toml --target-dir target/unsafe` (`clippy` the
  same); nothing to delete afterwards. (#1917)
* **A `tests/compiler` program that spawns `nikaia` finds it only under `nikaia test`**: its `toolchain()` takes the parent of its parent, which is `nikaia` only there; under `nikaia run` it falls back to the relative `target/fast/nikaia`, and `process::run(nikaia, ..; dir: tmp)` resolves that in the program's tmp dir, so a symlink in your working directory does not help (`no such file or directory: target/fast/nikaia`, #2060). Run it with `nikaia test --project tests/compiler --filter NAME`; a copy run by hand (a fails-before run, a scratch probe) gets the absolute path written into its `toolchain()` (and `root()`): `/home/user/Nikaia/target/fast/nikaia`, or `$S/old/nikaia` for fails-before. (issue-1333, issue-1414, issue-1665, issue-1705)
* **The compiler runs as rustc inside the program's `cargo build`**, so a sub-build in the same target dir waits for ever on Cargo's lock. Nothing says so: the nested `cargo` runs `--quiet` (`comptime_bundle.nika`, `grammar_run.nika`), so Cargo's *Blocking waiting for file lock* never shows, and `nikaia test` ends a test program's build only after `NIKAIA_TEST_BUILD_TIME_LIMIT` seconds (default ten times `NIKAIA_TEST_TIME_LIMIT`, so 3000 s; 0 is no limit; #1774, #1775): such a program then fails with `build timed out after N s` and a hint naming this `ps` check. A cold build or test silent for minutes is checked once with `ps -ef --forest | grep -B2 -E 'cargo (rustc|build) .*--target-dir'`: such a `cargo` under a `nikaia` that an outer `cargo build` started, both on one target dir, is this wait; kill it, do not wait it out. (issue-1705, #1727)
* **Inside a test program, `$PPID` and `/proc/self/exe` are the test binary (`.../target/nikaia/bin/program_test`), not `nikaia`**: a script that runs `$(readlink /proc/$PPID/exe) run ...` starts the test again, without end. Find `nikaia` as `toolchain()` in `tests/compiler/tests/nikaia_test_time_limit.nika` does (the parent of the parent, checked by its basename `nikaia`), or from `NIKAIA` where it is set. Such a fork storm filled `pid_max` and took the machine down (#1919); since then `nikaia test` caps a program's run at `NIKAIA_TEST_PROCESS_LIMIT` tasks (4096; the program fails with `ran into its cap of N processes and threads`) and ends its whole group when the runner ends, killed included. (#1919)
* A fix in our tool sources (not the compiler) has no fails-before run of the compiler; show instead the same test program written the old way reported checked. (issue-1496)
* A test parameter written `ref String` draws NK1261 (`ref` is redundant); write `String`. (issue-1496)
* A rebuild while `nikaia test` runs fails tests at random: rerun the failed
  ones alone before calling them failures. Under `--both-settings` it shows
  as `FAILED (the settings disagree)` on tests that pass alone (the settings
  ran different binaries), or as `Error: running .../nikaia (deleted)`; a
  test that spawns `nikaia` (every `drive_*`) runs its own ancestor, so run
  a filter of many such tests with your `$S/old/nikaia` copy. (issue-1414,
  #1837)
* Two `--lib` filters in one `cargo test` call (`emit:: sysroot::`) printed nothing for me; run one filter per call. (issue-1279)
* `cargo test -p nikaia-std --lib emit_core_args::` matches no test, because lowered modules have no unit tests; skip it for a `tools/*.nika` change. (issue-1290)
* Programs build against the workspace `crates/nikaia-std` source, so a fix in a std runtime `.rs` file passes on the old binary too; for fails-before, copy HEAD's version of that file back in for one run, then restore yours. (issue-1268)
* A two-file package test puts the struct in `tests/compiler/src/X_beside.nika` and the user in `X_beside_caller.nika`, and the program in `tests/` calls it as `compiler_tests::fn`. (issue-1272)
* Test programs had only `.stdout`, `.stderr`, `.stops`, `.bounds` and `.loops.bounds` expectations until `.asserts` (#1350); a new report expectation goes through `project_tests_bounds_outcome` and the host's `explained(args)`. (issue-1350)
* A change to the `check --asserts` header breaks `it` tests `assert::asserts_reports_every_claim` and `assume::asserts_lists_every_assume`, which match it exactly. (issue-1350)
* After an interrupted build, `cargo test --test it` can fail at link time with "undefined hidden symbol"; delete only `target/fast/deps/it-*` and `target/fast/incremental/it-*`, then rebuild. The same error linking `nikaia` itself: `cargo clean --profile fast -p nikaia-std -p nikaia` cured it (a 7.3 GiB rebuild); deleting only `target/fast/incremental/nikaia-*` and `nikaia_std-*` first is cheaper and untried. (issue-1366, #1671)
* `cargo test -p nikaia --lib prove::` and `--lib check::` run 0 tests; the prover is covered by the `it` tests `prove::`, `assert::`, `assume::` and by `tests/compiler` programs. (issue-1358)
* The LSP test programs in `tests/compiler/tests/` are already unformatted on HEAD; `fmt --check` on them proves nothing about your change. (issue-1361)
* In cargo test output, grep `test result:` with the colon; `^test result` also matches `it` tests named `result_source::…`. (issue-1310)
* `nikaia fmt` or `fmt --check` on a file outside a project formats with tabs, and the committed `tests/compiler` tests fail `--check` too; it proves nothing about a new test. (issue-1327)
* `fs::read` returns `Bytes`, so a `catch { "" }` on it is NK1103; in a test program, read text with `process::run("cat", …).stdout`. (issue-539)
* A test whose assert is refused (NK1300) does not build, so it cannot sit beside a `.stdout`: put the refused case in its own test with a `.stderr`. (issue-1375)
* When a mass migration such as `print-throws` rewrites test programs, the `accepted_sweep.rs` allow-list hashes change. The sweep's failure log pairs each new refused hash with the old "no longer checked" one, so replace the old hash with the new one. (qa-task)
* Running two `nikaia test` processes on `tests/compiler` at the same time can make `migrate_views3_reads_package_tests_with_the_package` fail with "cannot read TMP/json/tests"; it also fails alone on develop, so it is not your change. (issue-1334)
* No binary from before the views syntax exists (`target/release/nikaia` is absent); a fails-before run for a fix already on `develop` needs a build of the fixing commit's parent, so report it as not done. (issue-1356)
* Changing a help's wording re-records many `.stderr` files and the `lsp_diagnostics*.stdout` files; grep `tests/compiler` for the old phrase before building. (issue-1403)
* `nikaia test` run with a scratch copy of the binary fails `migrate_views3_reads_package_tests_with_the_package` (No such file); rerun it with `target/fast/nikaia` before calling it a failure. (issue-1460)
* New test programs go in `tests/compiler/tests/NAME.nika` (with `.stdout` and an empty `.bounds` before `--bless`), not in `tests/compiler/`. (issue-1470)
* A test for "rustc warns no more" needs an empty `NAME.stderr` beside `NAME.stdout`, since stdout alone passes on the unfixed compiler too. (issue-1420)
* A `.stdout`-only test passes even when the compiler prints a warning; add an empty `.stderr` beside it to pin "no warning". (issue-1491)
* `a_list_element_past_a_jump_is_refused_where_an_own_value_is_wanted` is red on develop before #1501 (the `by_val` NK1102 is missing), so don't chase it in a list filter run. (issue-1501)
* A scratch project for `nikaia test` needs a `src/` with at least one `.nika`, or the test run fails with "cannot read …/src". (issue-1236)
* `gh run view <id> -R Nikaia-Foundation/Nikaia --log-failed` reads a CI log read-only; a CI-only failure is traced from the whole failing shard, including its other failures. (issue-1584)
* Test programs that `git archive HEAD` (`drive_badges_head`) see only committed files, so they fail on a tree whose fix is not yet committed; rerun them after the commit. (issue-1600)
* A batch of test programs is driven by hand with `NIKAIA_TEST_BATCH=LIST nikaia build --project CARRIER` (LIST holds `NAME<TAB>ENTRY` lines, each program with its own `nikaia.toml`); `LIST.included` names what lowered, and `NIKAIA_LOWER_THREADS=1` makes it run in a fixed order. (issue-1639)
* Under `nikaia test`, a refusal in the tested package never reaches a batch: the batch's own package depends on the tested package, refuses first, and every program is then built alone. (issue-1639)
* The test runner's processes start in `ProgramHost::ran` in `crates/nikaia/src/project.rs`, not in `project_tests.nika`, and `std::process::run` cannot replace them because it returns text only (`NotText` on bytes). (issue-1672)
* **A plain string holding a child program's source draws `NK1111` only where a brace group names something of the outer file** (ADR-309 D15, by design, its accepted cost): `"{ran.get()}"` warns when the test file has a `fn ran` or a binding `ran`, while `"{finished.get()}"` beside it is silent. Write such a line as `f"{{ran.get()}}"`: in an `f"…"` a doubled brace is one brace, so it prints `{ran.get()}` with no warning; no rename. (issue-2235)
* A test program can now set `NIKAIA_TEST_TIME_LIMIT` (default 300 s; a build's limit, `NIKAIA_TEST_BUILD_TIME_LIMIT`, is ten times it unless set, 0 for none, #1775: a nested `nikaia test` with a tiny run limit sets the build's to 0) on its nested `nikaia test`, which turns a fails-before run of a hang into a few seconds on the new binary; on the old binary, wrap it in `timeout`. (issue-1672)

### Probes

Paths: `probe`, `reproduce`, `fails-before`, `develop`

* **One call: `nikaia run scripts/probe.nika -- SNIPPET`** (`--check`,
  `--file F.nika`) runs a snippet with `target/fast/nikaia`, no cargo, no
  `lower-std`; its output is the probe a report or an issue quotes.
* **Does the lowered Rust compile? `nikaia run scratch.nika`** on a scratchpad
  copy (b1006). `nikaia lower F.nika` writes `F.rs` itself; plain `rustc`
  cannot link `nikaia_std`; `nikaia run` refuses a file inside `tests/compiler`.
* **A wrong lowering: look before you read code** (b933): `nikaia check
  --tethers F.nika` prints what each result views, and the `nikaia.contracts`
  beside a lowered probe shows the ledger; they name the pass (`buffers.nika`,
  `tether.nika`, `views.nika`).
* **A checker defect "accepts X"** is probed in its neighbours first (a17): a
  user type, a std type, a literal, a typed and an open variable, on the old
  binary. Open numbers are typed by `number_where_no_number_is`.
* **To see a refusal of the prover**, put the call in a loop body or behind an
  input it cannot pin: a pure call with literal arguments is run at build time
  and proved (y4b).
* **A `simd` probe reads ADR-377 D3 first** (`nikaia run scripts/syntax.nika
  -- simd`): `Lanes[T]` offers only what keeps the output the same at every
  width, so `lane(i)`, a lane's index, is on `Fixed[T, N]` only, and a probe
  that calls it on `Lanes` stops at that name. (issue-1806, #1835)
* Before putting back a "workaround" an issue names, check `git log -S` on that line: a later commit may have removed it on purpose. (issue-1125)
* An issue's probe may pass on `develop` because a later fix masks it; vary the shape (index, `.iter()`, a second call site of another kind) before judging the defect gone. (issue-1210)
* A test of "a `keep` parameter keeps it" needs a callee that really keeps (stores or spawns); `fn hold(keep w: W) -> i64 { w.k }` only gets an NK1334 warning and proves nothing. (issue-1225)
* `nikaia lower F.nika` in the scratchpad writes `F.rs` beside it; read `__nikaia_main` there before guessing at an emitter cause. (issue-1289)
* A lifetime or borrow probe needs a non-`Copy` element type (give the struct a `String` field), or the emitter copies the element and the defect does not show. (issue-1228)
* Nikaia has no `loop` or `|x|` closures and declares mutables with `let mut`; write probes with `while true` and `fn(x) { … }`. (issue-1261)
* Calling code stored in a struct field with an owned `String` argument (`h.f(g)`) is a rustc error on `develop`; avoid it in probes until it is fixed. (issue-1264)
* A move probe needs a callee that keeps its argument (`all.push(x)` into a list declared outside the loop), or the checker lends the parameter and no move happens. (issue-1366)
* Nikaia's `a..b` includes `b`; use `0..<n` for n turns. (issue-1366)
* Run the issue's probe on develop first: a probe made only of Copy fields can pass there and hide the defect. (issue-1342)
* `nikaia check` accepts `HashMap[String, i64]`'s `insert(5, 7)` (std method arguments are not checked against bound types), so a probe that passes `check` can still fail in rustc; always `run` it. (issue-1345)
* An owned `String` in a probe is `let mut g = String()` plus `g.push_str("…")` or a `fn` returning `String`; `.to_string()`/`.clone()` on a literal is NK1301 and `String::new()` is NK1149. (issue-1356)
* A probe of ownership needs a `String` made by a call (`fn make()` with `push_str`), because a literal-filled `String` is tiered to `&str` and hides NK1343. (issue-1404)
* Before planning, also probe the inherent-impl and concrete-type variants of an issue's trait or generic probe: a trait method's by-value data parameter is seeded as kept in `keeps.nika` `trait_keeps` (pending #1012 step 3), so NK2105 there is not your defect. (issue-1448)
* When an issue's probe already passes on develop, find the fixing commit with `git log -400 --oneline | grep` on the topic, pin the probe in a test program, and report fails-before as not done. (issue-1458)
* A match arm can't be an assignment (`null => x = y` fails to parse); write `null => { x = y }`. (issue-1478)
* `collections::BTreeMap` has no `extend` (NK1171), and from a function reached by a `sync` caller the refusal first shows as NK2202 "can pause"; mark the callee `sync` to see the real error. (issue-1478)
* A list whose items come from `split()`/`map()` is a list of views of the argument, so a probe of "owned items" must build them with `a + "…"` or `push_str`. (issue-1481)

### Checker and views

Paths: `check_`, `checker`, `text_tiers`, `tiers.nika`, `view_changes`, `keeps.nika`, `ref String`, `views-3`, `emit_`, `emitter`

* **One `keep` anywhere in a file turns `NK2302` on for the whole file**
  (v9-v11): a probe for a views change carries one `keep` function.
* **A view-checker refusal (NK2302, NK2106, NK1106)**: narrow it in two probes,
  the value as `return x` and as `let r = x; return r`, with the field really
  a view (built from a `trim()` in a function, not a literal in `main`) (y21).
* **`Self` in a trait's signature is `Ty::Named { name: "Self" }`**, not a
  `Ty::Var`; `substitute` (`ty.nika`) does not replace it. A call through a
  bound is typed in `check_core_call_on.nika` via `bound_that_answers`
  (`check_decls.nika`); `Eq`, `Ord`, `Clone` are `STANDARD_BOUNDS` (v15).
* **Kinds in a `match` on a kind** are Part II 10.3's (`I64`, `F64`, `Text`,
  `Bool`, `Maybe(A)`, `List(A)`, `Map(K, V)`, `Struct(S)`); the catch-all is
  `else` (j2).
* **A loop that takes the elements away** is `for x in v.drain()`; `for x in v`
  over an owned `Vec` only borrows them.
* **Print the `Ty` the checker assigns before writing a predicate on it**
  (v18b): the text view is `Ty::Named { name: "str", view: true }`, not a
  `String` view; own copying enums are lent as `&T`.
* **An `Array[T]` parameter is decided in five places** (v21): lending in
  `lends_with` (`lends.nika`), the length generic in `emit_function.nika`, the
  `&` at a call in `check_arguments.nika` (the lent rule), `&mut` in
  `lent_to_a_mut_parameter` (`check_references.nika`), a part's length to the
  prover in `length_given` (`prover_state.nika`).
* **The argument check is `check_arguments.nika`**; several early `continue`s
  there (about line 353, a view handed to a lent `String`) skip
  `the_compiler_writes_the_reference` (v18b).
* **A change that flips a tool function's signature breaks its Rust callers**
  in `crates/nikaia/src` and `tests/it`: `grep` for them before the bootstrap
  (v18b).
* `let c: ref Array[T] = v` (or `= x.field`) of a `Vec` local moves it (NK2105 on the next use); only an index or piece (`v[1..]`, `v[0]`) lends. (issue-1270)
* A by-value parameter handed back is marked `=x` (owned text, no refusal, ADR-282 D12) in a free function, but `!x` (NK2302) in an `impl` of a trait whose result writes `from`, because the contract follows the trait. (issue-1280)
* In that `impl` case the checker also gives NK1104 with the help "write `ref`"; following it leaves only NK2302, and `it` test `views_handed_back::following_nk1104s_help_is_a_program` checks that help word for word. (issue-1280)
* `unviewed_type` of the text view must give owned `String`, not `str`, or `.cloned().collect()` comes out as `Vec[str]`. (issue-1388)
* The views pass marks a by-value parameter `!` (NK2302) only when its type is not textless (`rsrc_textless`); a struct of only numbers gets no NK2302, so a checker refusal of it must stay. (issue-1386)
* NK1104 for a `return` and for a function's last value both go through `expect_kept_asked` (`check_arm_leaves.nika`) with `what == "returns"`, so one guard there covers both. (issue-1386)
* NK1104's help still says "write `ref`" (`way_out_of_a_mismatch`, `check_types.nika`), and `return ref s` of a local in a trait impl with a `from` result is an NK1343 internal error. (issue-1386)
* Under views-3, NK1131 fires only on bodies that mix a view path with an owned path, on `for`-binding roots and on `mut`-parameter roots; `from` gives NK1104 on all three, so a help must not offer it. (issue-1403)
* A nullable result written `from self` is still NK1131, and `-> ref U?` on a method is NK1343. (issue-1403)
* A check run over `value_leaves` (or the `block_tail` leaves of a choice: a function's or a lambda's last value) after the arms are checked must not re-type a leaf with `check_core_expr` unless its root is a parameter (`a_place_of_a_lent_parameter`), or names a `match` pattern bound give NK1117 in `ast.nika` during `lower-std` (`comptime_tree.nika`, `emit_reach.nika`). (issue-1425, #1911)
* A `pub fn` with a mixed `String` result keeps it owned (ADR-282 D22), so a borrowed-field arm needs the per-arm NK1131; probe both the tail form and the `return if` form. (issue-1425)
* `CheckFacts.method_callees` holds the ledger key of **every** described method call, `std`'s too (`Bytes::text`, `str::trim`, `net::Connection::read(within)`), filled in `check_core_call_on_described`, not only the user's own methods its comment's `Holder::put` suggests; `!method_callees.contains_key(on_node)` therefore excludes `std` calls as well. Tell a user method from a `std` one by the key's type part (`starts_with("str::")`, `== "Bytes::text"`, as `emit_core_calls.nika` does) or by `std_receivers`. (issue-1870, #1900)
* The tether's keep plan (`tools/buffers.nika`, `is_a_buffer` in `tether.nika`) treats only text and bytes as buffers, so a view of a list or struct local handed back is NK2304 (ADR-283 D9), never NK2303, which is not a refusal. (issue-1155)
* A `let` over a field of a `ref` parameter (`let k = index.keys`) is NK2106 by design; read the field in place at each use. (issue-1279)
* A rustc "temporary dropped while borrowed" or "mismatched types" on text usually comes from the text-tier pass (`text_tiers.nika`, ADR-282), not the keep code: lower the probe and check whether a `Vec[String]` became `Vec<&str>`. (issue-1287)
* `.to_string()` of text is emitted as the bare receiver (`text_as_is`, ADR-282 D8); copying text out of a view is `.clone()`, and a kept view of a temporary is NK1102 by design. (issue-1287)
* Typing a path expression `T::X` (variant, method value, `T::fields`, integer limits) is in `check_core_expr_path` in `tools/check_core_expr.nika`. (issue-1294)
* Nikaia's integer types are only `int`, `i32`, `i64`, `u8`, `u32` and `u64` (Part I 2.2); `integer_named` in `check_names.nika` tests for the fixed widths, and `int` is rewritten to `i64` only in emitted types (`emit_core_types.nika`), not in paths. (issue-1294)
* A `from` fix for text must also be probed with a struct type: the trait lowers only text `from` as a view, so a fix in `keeps.nika` breaks the struct case. (issue-1250)
* A text `+` is a `Binary` node that the emitter recognises by `facts.concatenations.contains(span.start as i64)`, using the operator span, not `node_key`. (issue-1251)
* A `let` bound by `x ?? null` used to lose `x`'s `ref` (it was typed as the `null`); if a `&` appears in front of a `match` in the lowered code, check the checker's result type of the `??` first (`check_core_ops.nika`). (issue-1340)
* a view of a lent parameter handed to an owning `String` result is a refusal with a `.clone()` hint by ADR-282 D8 and ADR-views-v4 D12/D14, whatever an issue's expected output says. (issue-1263)
* A text view in the checker's `Ty` is `Named { name: "str", view: true }` (printed as `ref String`), and `is_a_view_of_a_string` misses it; match `TEXT_VIEW` too. (issue-1264)
* A refusal of a view handed back where `String` is wanted must exempt static text: ask `text_literal(world, a_string_type(), value, true)`, or `"x"` is refused too. (issue-1264)
* A `let x: Array[T]` (no length) is made a view by `lv_let_views` in `text_tiers.nika`, like fields (`sf_slice_fields`) and results (`hb_fn`); extend those passes rather than the checker. (issue-1270)
* Package-wide facts reach a file's tier pass only through `BesideFields` in `crates/nikaia/src/text_tiers.rs` (built in `beside_fields`, read by `refine_with`); add a field there rather than a new parameter at each caller. (issue-1272)
* `cargo test -p nikaia --lib text_tiers::` runs 0 tests; the tier pass is covered by tests/compiler programs and by the `it` tests `text_tiers::` (`crates/nikaia/tests/it/text_tiers.rs`). A checker change to lending or moving (`hands_over`, `parts_lent`, `lent_args`) is a touched test of `cargo test --profile fast -p nikaia --test it -- text_tiers::` too: #1740's first fix broke `m[line]` of a view there, and no program caught it. (issue-1272, #1753)
* NK2105 for loops is decided in `hands_over_path_of` and `CheckRepeats` (`check_takes.nika`, `check_state.nika`); `revived` there also drives narrowing (ADR-382), so a new exemption needs its own set. (issue-1366)
* A value moved inside a loop is no NK2105 when the moved name gets a new value before the turn ends: `let mut y = p` before the loop, then `use(x); x = y; y = [2]` in the body brings a parameter to `x` only on the second turn without a clone (`rust_owner_rebound_to_a_copy_is_followed`). (issue-1713)
* NK1111's doubled-brace and hole warnings are both in `unmarked_hole` (`tools/check_host.nika`), and the brace scanning helpers are in `tools/check_words.nika`. (issue-1361)
* A `let` of a choice mixing owned text and a view is typed `ref String` by the checker but emitted as `EitherText`; a missing `&` at a call is a `lent_args` question in `check_arguments.nika`, not an emitter one. (issue-1310)
* `.to_string()` of a text slice is a view in the checker as well (NK1105 when assigned to a `String` local), not only in the emitter. (issue-1310)
* The checker has two text-view forms, `Named{"str", view:true}` and `Named{"String", view:true}` (the latter from `ref $T` with T=String, e.g. `.iter()` and `or_insert`), and both print as `ref String`. (issue-1316)
* Do not merge the two text-view forms in `ty.nika` `substituted`: the `String`-slot form means `&String`/`&mut String` to the emitter, and writes through `or_insert` break; normalise in the one check that needs it. (issue-1316)
* An NK1103 that says "is `ref String`, but the `let` declares `ref String`" is the two view forms meeting, not a tier question. (issue-1316)
* A `from` fix in a trait must change both sides together: the impl's types in `hb_trait_from` (`text_tiers.nika`) and the trait's in `emit_item_lent_view` (`emit_item.nika`); one alone gives rustc a trait/impl mismatch. (issue-1327)
* "file NNN" in a `Found:` of the #1254 runs means `tests/accepted/NNN_*.nika` migrated by dropping `ref` from its parameters; reproduce on that file, not only on the reduced probe. (issue-1334)
* `container_of` in `tiers.nika` returns null for `ref` types, so the tiers pass does not follow a `ref Vec[...]` parameter, and its `String`s stay `String` below (`lent_whole`). (issue-1449)
* The checker types `b[a..<c]` on a `Bytes` as an owned `Bytes` (ADR-320 D7), but the emitter writes a `&[u8]`; the fact `bytes_cuts` marks such a cut. (issue-1345)
* `check_crossings_arguments` gets the unbound contract, so `want` there is the type variable `K`; the substituted types are `expected`/`bound` in `check_core_call_on`, and `entry`'s key is `?` (use `bound["K"]`). (issue-1345)
* Turning a move into a lend in the checker also needs `vc_view_of` (`view_changes.nika`) to see the `let` as a view, or a later write to the source passes the checker and becomes a rustc NK1343 internal error. (issue-1362)
* A `let` bound to a cut (`v[0..]`) or a `std` method result (`s.trim()`) is a view only if `vc_view_of` in `view_changes.nika` sees it; otherwise a later change becomes a rustc NK1343 instead of NK2305. (issue-1363)
* The library ledger has no `borrows` entries for `std` methods (`String::trim` is missing from `described.contracts`), so the view-change walk cannot tell that a method result points into its receiver. (issue-1363)
* The tier pass runs without checker types, so a rule there must not mark lists whose items are already views (split/map) as owned. (issue-1368)
* "Copied at a read" is `a_word_by_its_kind` (numbers, bool, char, scalar, payloadless enums, ledger `copies`); `a_copy_by_value` covers only the first four, and a copy-out keyed on it misses `Symbol` and enums. (issue-1460)
* A `let x: V? = m.get(k)` or `m[k]` is typed `ref V?` by #297's `of_a_map` block in `check_core_stmt.nika`; a value-typed read must be exempted there. (issue-1460)
* Checker facts (`check_facts.nika`) are pure Nikaia with no Rust mirror: a new field needs only its declaration and its initializer there. (issue-1411)
* An impl method's contract is built by `declared_function` (`declared.nika`) from `ledger_items.nika`; a generic impl is written `impl Box[T]`, never `impl[T] Box[T]`. (issue-1448)
* Views-3 D12 (returned parameter is a view) is built only for text; a struct or generic `T` result still keeps its argument (`fn second<T>(x: &T, y: T) -> T`). (issue-1448)
* A new `VcAsk` field also needs the Rust adapters `view_changes.rs` `analyse`, `check/host.rs` `view_change_findings` and `contracts/keep.rs` `buffer_plans`, plus `emit_contracts.nika` and the `check_walked.nika` trait. (issue-1478)
* `views_what_it_is_called_on` (`tiers.nika`) is the existing list of text methods whose result is a view of their receiver; reuse it, don't write a new list. (issue-1478)
* A `-> ref(a) String` or `a: borrowed` in the ledger for a `String` parameter comes from the text-tier pass (`lendable_parameter`, `a_keeping_target` in `text_tiers.nika`), which runs before the keep inference and the checker. (issue-1491)
* Files of the views-3 migration may carry `keep` marks the old binary already warned NK1334 on; when your fix makes `keep` mean kept, drop such marks where the file means lent. (issue-1491)
* A local is put into a keep (`put_viewed`) only when a view of it leaves the function, e.g. `let v = parse(text); return v`; a plain user program with a map lookup never gets a keep frame. (issue-1073)
* The emitter's `walk.kept_names` maps each kept name to its `let`'s start, and `emit_stmt_block_plain` restores it at the end of each block; a new binder that shadows a name needs no special handling, because the checker's `declared`/`bound_at` facts resolve each use. (issue-1073)
* `CheckFacts.declared` and `bound_at` (#1396) are filled on every build, so the emitter can resolve a variable use to its binding's span through them. (issue-1073)

### Lowering and lower-std

Paths: `lowered/`, `lowered.rs`, `lower-std`, `sysroot`, `lower_mine`

* **Check lowered tools code with `cargo check --profile fast -p nikaia-std
  --features compiler --message-format short`** right after the first
  `lower-std`; without the feature the check is a false green (k3a2).
* **After any `*.nika` or `described.contracts` change**, run `lower-std` with
  a binary built from that tree and commit the diff; it reads and writes the
  checkout the binary was built in (set `NIKAIA_SYSROOT=/home/user/Nikaia/crates`
  for a binary built in a private copy). It lists every refusal of a file: fix
  them all before the next cycle. A new `.nika` needs one `lower-std` run
  before `cargo build`, and a row in `scripts/self_hosting.py`.
* **A retype of a lowered type needs a probe per shape before the first `lower-std`**
  (v28): text, struct, tuple and generic `T` are spelled apart in `emit_core_types.nika`;
  one probe each on the draft binary saves a ten-minute bootstrap cycle. The before-binary
  is a fresh build of HEAD: after a cut run, `target/fast` may hold the last draft.
* **`keeps.nika` (`lends_with` in `lends.nika`) drives both the view migration and
  what the tool files lend** (v24): a change to the keep rule rewrites how 20 or
  more tool files lower. Probe first that they still compile under the new rule.
* **A keep or lowering rule change bootstraps in two stages** (v18b): keep the
  binary from before the change (`cp target/fast/nikaia $S/old-nikaia`). After
  editing the rule: build, then `lower-std` with the new binary and `cargo check`
  (above). If the lowered files do not compile, restore them by name, fix the
  rule or the tools `.nika`, and never run `lower-std` again from a binary that
  was built from lowered files that do not compile: start over from the old
  binary.
* **Reproduce a lowering defect in seconds**: `nikaia lower f.nika` or `nikaia
  run f.nika`. One that shows only in `tools/` needs a throwaway function in a
  tools file and `lower-std`; restore both after.
* **A lowered clippy lint** (`.map(fn(x) { x.clone() })` trips `map_clone`) goes
  into the allow list of the `tools` module in `crates/nikaia-std/src/lib.rs`,
  never into the lowered file (P5d).
* **A package is lowered file by file** (cv2): probe a new mutual call between
  two files as a two-file scratch package. `code_on_a_cycle` is not
  package-wide yet (#1129).
* **The lowering cache misses an inferred `sync`** (#1014): declare a function
  `sync` when a body change could make it `async`. Remove when #1014 closes.
* **A build-time value fails with `NK1127` and no reason**: the sub-program is
  `<project>/target/nikaia/build-time/comptime/<hash>/main.rs`. A new
  `comptime` value shape must survive as a literal (`comptime_literal_of`) (j5).
* **A `std.contracts` method Rust has under the same name** needs only the
  contract; the file is built into the binary, so rebuild after editing it.
* An `Item::Fn` name is `Symbol?`, so `ask.names.resolve(name)` is refused (NK1102); use the plan's `key` instead. (issue-1155)
* A later call argument that reads a place lent `&mut` is hoisted into a `let` (`emit_call_hoisted`); only place reads and lambdas reach rustc, so a check on arguments must leave out what the lowering hoists. (issue-1209)
* `check_visit_stmt` returns every subexpression (so `search` inside `search.proof`), and `check_visit_children` does not descend into lambda bodies; a place-path check needs its own top-level walk plus `check_visit_expr_blocks`. (issue-1209)
* `String: Plus<EitherText>` from rustc means a tiers defect made a `String` parameter mixed; the checker never allows that itself, so fix the tiers, not `concat.rs`. (issue-1210)
* A buffer put into `KeepAt::Param` needs an `Escape::Returned` entry in `plan.escapes`, or the ledger never marks the function as taking a keep and `__keep` is missing. (issue-1218)
* `migrate views-3` works on tokens and runs `migrate_views3_code` twice (once to drop `ref`, once to write `keep`), so a rewrite keyed on `ref` must still hold on the already-rewritten text. (issue-1224)
* In `keeps.nika`, a value handed back goes through `hand_over`/`reached_of`, so anything added there also keeps on `return`, which D8 forbids; put store-only escapes on `Stmt::Assign` instead. (issue-1225)
* Thread-locals in proofs, comptime and project make a parallel check safe only in lower-std (`infer_package_parallel`), never in project builds. (issue-1277)
* `lower_compiler_files_cached` re-parses files with `test` blocks (`with_tests_as_functions`), so package-wide values computed from `parsed` do not cover those files' test functions. (issue-1279)
* A `__keep_N` in the wrong scope comes from the keep plan in `buffers.nika` (`decide_source`, keyed by `walk.statement`), not from the emitter. (issue-1289)
* A trait method's positions are published in the text tiers (`of_a_trait`), so a view that one side of a trait/impl gets from `emit_item_lent_view` must be mirrored into the impl's AST (`hb_trait_from`), not inferred. (issue-1250)
* Setting `ty.is_view` on a `String` in an AST rewrite makes NK1261 warn about a `ref` the user never wrote, unless the position's key (`param K n` / `result K`) goes into `parsed.text_views`. (issue-1250)
* A `String` param that a mixed result hands back through `let o = own(a) ?? return null` lowers as `EitherText` by value, which gives a rustc move error, not an NK refusal. (issue-1254-r1)
* A `for` over a `Vec[String]` place lowers to `.iter()`, so its element is `&String` below; only numbers, `bool`, `char` and `&str` are copied out with `let x = *x`. (issue-1268)
* A buffer `let` is put into the keep by `emit_stmt_kept` (`plan.puts`, `emit_stmt.nika`), decided in `buffers.nika` `decide`, and for non-text lists in `plan_of` via `untethered_returns`. (issue-1276)
* A buffer changed after its `let` now carries `late_puts` and is put by a shadowing `let` after its last change; the reassignment puts (#293 reputs) before that point are dropped. (issue-1276)
* `std` contracts write `fn(T)` for code whose result they leave open (`Seq::filter`), so a `Ty::Fn` with `result == null` means "hands back nothing" only for the program's own types. (issue-1321)
* A lambda body is written in several places (`emit_core_flow` plain, failing and kept paths, and the pausing path in `emit_core_args`), and an emitter fix must cover each one. (issue-1321)
* The emitter's `world.library` is std only; to know a dependency's errors, ask `name_of_error` (it checks `own_contracts.types`), not `error_kind`. (issue-539)
* Whether an argument to an `EitherText` parameter is lent or moved is decided by the late-read walk in `text_tiers.nika` (`late_argument`, `late_args`), not by the emitter. (issue-1334)
* A list of string literals is tiered to `Vec<&'static str>`, so `ref $T` binds `T` to a view and a walk gives `&&str`; `AsRef::<str>::as_ref` treats `&str`, `&&str` and `&String` alike. (issue-1388)
* The emitter knows a call borrows its root mutably through `emit_lends_mut` (facts `mut_receivers`/`mut_args`); use it rather than a new checker fact. (issue-1445)
* An index *write* already hoists a position that names the target's root (#570); an index *read* is `emit_core_index_read`, and it hoists inside a `*{ let ..; get(..) }` block. (issue-1445)
* Rust generics get no deref coercion, so an argument at a lent type-variable parameter gets exactly one `&` (`&&C` does not satisfy `C: Bound`). (issue-1342)
* In the checker state a type parameter is `Ty::Named` (listed in `state.type_parameters`), not `Ty::Var`; `Ty::Var` appears only in contracts and signatures from `parameterise`. (issue-1342)
* Under `impl<'a>` (`EmitLifetimes::Inner`), `EmitLifetimes::Named` means the impl's `'a`, so a store into self ties the source to it without declaring a new lifetime. (issue-1344)
* `ref Array[T]` is `Ty::Pointed { slice: true, mutable: false }` in the checker, not `Named { view: true }`, and a `Vec` fits it, so `!found.fits(want)` guards never fire for it. (issue-1362)
* In the checker, a text view's base name is "str" (shown as "ref String"), so a test on `name == "String"` misses it. (issue-1368)
* `check_core_returns` decides `lending` before `check_core_expr` runs, so a fact recorded while checking the value must be read after `found`. (issue-1368)
* A plain `let` over a local's field is lent by the emitter (`lent_lets`); its binding type must be a view too, or a later keeper becomes NK1343. (issue-1404)
* A function's result lifetimes (`'static` widening, `'e`, `'r`) are chosen in `emit_function.nika` around `borrows_from_something` / `widens`; `emit_core_types.nika` only writes the `EmitLifetimes` it is handed. (issue-1420)
* Buffers (`buffer_plans`) see only the items of one file, while the tier pass sees package facts only through `BesideFields`, so a shape that works within one file can be rustc NK1343 when the callee is in another file. (issue-1481)
* An open number with no `let` to write its type on (map parts, list literals walked in place) has its type written as a suffix on every literal; skip `number_lets` for it, or the emitter has nowhere to put the type. (issue-1485)
* A tuple literal's parts share one text kind in the tiers, so lending one part changes the kind of the others (it changed the `buffers.rs` map key to `EitherText`). (issue-1490)
* A local's `from` is set for a call result too; to test "this local views another", add `!local.made`. (issue-1490)
* In the text-tier walk, a rule that reads `walk.adopted` must insert into `walk.reads.adopted` without a condition, or the round that adopts never walks the unit again. (issue-1501)
* `names_in` counted a struct-literal field label as a read of a local of that name. This is fixed in ba1ec3be; a stray `let v = v;` in lowered output points at this kind of false read. (issue-1501)
* The lowering cache of a program lives in `PROJECT/target/nikaia/cache/`, and its key changes with every compiler rebuild, so a planted entry must go in after the last build. (issue-1582)
* **`lower-std` lowers the other agents' unfinished `.nika` too**: one half-written can refuse your cycle (an NK code whose `-->` names a file `git status` shows modified and not yours), and their `lowered/*.rs` change under your cycles. Save the list once before the first cycle (`git status --short crates/nikaia-std/src/compiler > $S/before.txt`); a refusal or a lowered change in a file on it, or new since, is theirs: no defect, no `Found:`, no fix, never staged (`commit.nika` adds only your `Files:`). Judge your change by your own files and the new binary's `lower-std`; name the refusal once under `Tests:`, not as friction. Such a refusal makes the tree's `lower-std` write no file at all, yours neither: `nikaia run scripts/lower_mine.nika -- YOUR_FILES...` lowers HEAD plus only your files in `target/lower-mine` and names each lowered file your change moves (`--write` copies into the tree only the lowered files of the FILEs given, never `lowered.rs`, and names the others it moves `not written`, #1918; `--exe $S/old/nikaia` for the cycle's first run) (#1875). Their `.nika` may also lower to Rust the build refuses (a rustc error in `lowered/X.rs` of a file on the list): for each build, put HEAD's copy of only that lowered file back (`git show HEAD:PATH > PATH`, the one sanctioned exception to never restoring `lowered/` by hand; their next `lower-std` writes it again), never stage it, and name it once under `Tests:`, not as friction or `Found:`. (issue-1759, #1762, #1794)
* After a `.nika` change, the cycle's first `lower-std` must run with the before binary (`$S/old/nikaia`); `target/fast/nikaia` still holds the logic of the previous attempt. (issue-1209)
* `NIKAIA_LOWER_CACHE_SALT=<fresh>` plus `lower-std --sysroot <copy>` gives an uncached reference run to `diff -r` against for byte-identity, without touching the shared cache. (issue-1277)
* Whether HEAD lowers a file as committed is shown by that uncached run on a `git archive HEAD | tar -x -C $S/head` copy, diffing the copy's `lowered/` against `git show HEAD:...`; a plain `git diff` of the shared `lowered/` also shows the files other agents lowered mid-run (`bounds_rules.rs`, `bounds_walk.rs` in #1763's run). A `Found:` from such a run names HEAD's subject and sha and the binary's, so the next run can bisect once the tree moves on. (issue-1763, #1793)
* `nikaia check` on a single `crates/nikaia-std/src/compiler/*.nika` fails with NK1135 for imported types (`Span`); only the `lower-std` cycle checks a tool file. Its first step needs no build: after each edit of a tools file run `$S/old/nikaia lower-std` alone (seconds to a minute, one file re-lowered), and fix what it refuses before the 5-minute build (`let mut v = ""` later given a `String` is NK1105 by ADR-282; start it as `String()`). (issue-1289, #1764)
* The cycle's first `lower-std` with `$S/old/nikaia` can fail on files nobody changed; the new binary's `lower-std` is the one that counts, and that failure goes in the report. (issue-1228)
* When the before binary's `lower-std` fails on files nobody changed, put HEAD's version of your own `.rs` files back, build, lower, then restore them and build again. (issue-1350)
* A checker change that makes a tools file fail to check must be lowered with the old binary first (`$OLD lower-std`), or the change never reaches `lowered/`. (issue-1388)
* The first `lower-std` with the old binary resets lowered files a broken build left behind; after a failed build, restart the cycle from the old binary, not the new one. (issue-1344)
* `lower-std --sysroot` takes the `crates` directory of the copy (`X/crates`); given `X` it fails at once with "reading X/nikaia-std/src". (issue-1473)
* Never pipe the cycle's two `lower-std` runs into one grep without a marker; you cannot tell which binary raised the error, so echo a label between them. (issue-1425)
* Moving `crates/nikaia-std/src/compiler/` means editing `SYSROOT_COMPILER` in `sysroot_cache.nika` and in its lowered `.rs` by hand before the first build, because the old binary cannot lower from the new path. (issue-1600)
* The `__nikaia_tupleN` names in lowered files are byte offsets, so any change to a `.nika` comment's length renumbers them in the lowered output; that is not a behaviour change. (issue-1600)
* `lowered.rs` changes on every `lower-std` after a `.nika` edit (`LOWERED_DIGEST`): commit it with your lowered file, and check stability by comparing it before and after the second build. When the tree's `lowered.rs` also holds another agent's `include!` line, list it under `Partial:` with your hunks staged (the digest hunk at least): `commit.nika` refuses a staged `include!` of a lowered file the commit lacks and a lowered change without its `lowered.rs`, and writes the digest of the set as committed into the staged copy itself. (issue-1657, #1709)
* A call in `crates/nikaia/src/main.rs` or `sysroot.rs` to a function a `.nika` change adds does not compile until `lower-std` has written it: write the `.nika`, run `lower-std` with the old binary, then add the Rust call and build. (issue-2082)
* A dependency of std (logic, json) is lowered into `lowered/<name>/` with its own index, so a list gathered across std (its tests, say) must also fold in each dependency's list under its name. (issue-2082)
* **A lowering fix and the first std code that needs it are built in that order**, never in one cycle: the old binary's `lower-std` writes the new form (a std test block that pauses, say) as Rust rustc refuses (E0728 in `run_config.rs`), so `nikaia-std` does not build and no new binary comes. Write the fix in the compiler first, cycle (old binary's `lower-std`, build), and only then write the `.nika` that uses the form and run the new binary's `lower-std`; a draft that already holds both sets that code aside for the first cycle and puts it back after. One cycle saved. (issue-2316, #2365)

### Grammar

Paths: `grammar`, `parser`

* **Check the baked winnow-grammar rev before the first probe** (y12): `grep -c
  "$(grep -o 'rev = "[0-9a-f]*"' Cargo.toml | grep -o '[0-9a-f]\{7,\}' | head -1)"
  target/fast/nikaia` prints 1 or more; if 0, `touch crates/nikaia/build.rs`
  and rebuild. Otherwise rustc says "multiple different versions of crate".
* **A grammar probe to copy** (y2b). It prints `42`; the generated Rust is
  under `~/.cache/nikaia/run/main-*/target/nikaia/gen/` (`ls -dt`). A lowercase
  rule skips whitespace before each element; `rule WS = "" -> { }` turns that off.
  ```nika
  grammar G {
      rule WS = "" -> { }
      entry rule doc(n: i64) -> i64 = d:dec[i64](digit+) eof -> { d + n }
  }
  fn main() throws {
      let input = Bytes("41".bytes().collect())
      let r = G::doc(input, 1) catch {
          println("error")
          return
      }
      println(f"{r}")
  }
  ```
* **A rustc error from a grammar** is the checker's or the emitter's: run
  `nikaia check` and `nikaia lower FILE` first (y9). A label's type comes from
  `CheckState::pattern_type` (`check_grammar.nika`); a rule's tier also from
  `grammar_pattern_kinds` (`text_tiers.nika`): a new pattern kind needs an arm
  in both (y10). A guard: `bind_grammar_pattern` (#1032) and the
  `Pattern::Guard` arm of `emit_pattern.nika` (#1033); probe it alone (y2c).
* **`peek`, `not`, a cut, a guard and `fail` cannot be bound.** A published
  field may not hold both a view and owned text (ADR-282 D22). `nikaia fmt` can
  break a rule line holding `if(cond, "label")` (#1015): check it still parses.
* **Open grammar defects** (`ready`; never written around): #1001 branches of
  different types reach rustc; #1002 `hex_digit` undefined; #1004 a temporary
  `Bytes(..)` as an entry argument. An option after `;` needs a default; a
  helper called from an action is `sync`. Remove a line when its issue closes.

### Packages

Paths: `nikaia.toml`, `nikaia.lock`, `nikaia.proofs`, `ledgers`, `packages/`, `examples/`, `artifact`, `nikaia build`

* **A test that reads another package's type**: `tests/compiler` is the package
  `compiler_tests`; its `pub` items in `src/main.nika` are reached with `use
  compiler_tests` (#1037).
* **A package commit** holds `nikaia.contracts` and `nikaia.derived` when they
  changed, never `nikaia.lock` or a `nikaia.proofs` you did not cause. Write a
  new test's `.nika` before the first `nikaia test --project` run (3-4
  minutes) and `--bless` in that run, only once `nikaia check` is clean: a
  package that fails to build blesses a `.stderr` for every test (y21, y27).
  An `NK1127` on a default after your change: `nikaia run` a scratch copy with
  the defaults written out to see rustc's error.
* **`nikaia run` takes the run_fast path without Cargo**: probe a build-time change with a cold `nikaia build`, not `nikaia run`. (issue-1705)
* With `cargo --target`, proc-macro crates land in `<dir>/debug/deps`, not `<dir>/<triple>/debug/deps`. (issue-1705)
* **A tracked `nikaia.lock` is a ledger, stable across builds** (#1945): it
  holds the toolchain, the units' source hashes and the resolved dependencies,
  not the compiler's identity (ADR-021 D2, D3: that is in the cache key under
  `target/`), so a rebuild of the compiler leaves it as it is. Most locks are
  git-ignored; `packages/*/nikaia.lock` and `nikaia.proofs` are tracked.
  Commit the lock with a change to its package's sources; one you did not
  touch that changed in the tree comes from the driver's `drive_ledgers` or
  another run's `ledgers.nika --refresh`, which commits it: leave it
  unstaged. `nikaia run scripts/ledgers.nika -- --check` says whether your
  change made a ledger or a lock stale (issue-1691, #1945). A stale one is often stale on
  `develop` already: one run of `ledgers.nika -- --check --tree HEAD --exe
  $S/old/nikaia` (HEAD's ledgers, the before binary) settles it; stale there
  too, it is not yours, say so under `Tests:` and do not refresh it (#1712).
  `ledgers.nika --check` and `--refresh` build every ledger directory
  (`-- --list` names them, 19 today) cold, in a fresh `NIKAIA_CACHE_DIR`:
  many minutes, past the tool's foreground limit. Run it with
  `run_in_background` and its output in a file (`> $S/ledgers.txt 2>&1`),
  never piped through `head`: its output is already short (a line per
  directory, 40 per failure, `ledgers_head`), and a foreground run piped
  through `head` was killed (exit 144) and ended the session (#1777).
* A defect that shows only when the call crosses packages is often a type `qualify` (`ty.nika`) leaves unqualified, so a dependency's `H` never matches the caller's `lib::H`; check that before the lend logic. (issue-1417)
* A cross-package probe or test copies the `mktemp` two-package setup of `tests/compiler/tests/dependency_struct_in_a_hole.nika` and runs `run` in `{tmp}/app`. (issue-1417)
* Files of one package share one namespace: call a sibling file's `fn` by its bare name, private or not (`mod x` is a parse error). Mark a `fn` `pub` only where another package or the Rust host (`crates/nikaia/src`, `crates/orchestrator`) calls it: the bounds walk derives no precondition for a `pub fn` (ADR-391 D4), so a needless `pub` leaves its sites checked. (issue-1481, #1831)
* `nikaia cache prune` counts only directories two levels down (`<kind>/<key>`) that hold a `.nikaia-used` stamp; a test that builds in an unstamped directory grows the cache unseen. (issue-1469)
* A `nikaia cache prune` test sets the entry's age with `touch -d '3 days ago' <entry>/.nikaia-used` and points `NIKAIA_CACHE_DIR` at a temp directory; on unix an entry used recently is still removed, unless a build holds its lock. (issue-1469)
* A `--locked` run in place uses the package's own `target/nikaia/cache`; to search every proof again, run a copy of the package without `target/` and with fresh `NIKAIA_CACHE_DIR` and `CARGO_TARGET_DIR`. (issue-1513)
* A `--locked` flake on an example is usually a stale `nikaia.proofs` hidden by a warm cache; check `git log -- examples/X` after the failing run before suspecting timing (the prover's `LgBudget` counts steps, not time). (issue-1513)
* A toolchain at another path costs nothing to make: `cp -al $(rustc --print sysroot) $T/.rustup/toolchains/NAME`, run with `RUSTUP_HOME=$T/.rustup RUSTUP_TOOLCHAIN=NAME`, works on the same filesystem. (issue-1584)
* The run-bundle record in `NIKAIA_CACHE_DIR` is shared by every nested `nikaia run`, so a test that changes HOME or the toolchain leaks into the other tests through it. (issue-1584)
* Building a copy of an example with path dependencies needs a symlink from `crates` to the repo's crates; a copy gives cargo's "package collision in the lockfile". (issue-1591)
* A package with no `fn main` needs `artifact = "rust-library"` in its manifest; `build` and `check --rustc` refuse it otherwise (NK1351), and `scripts/ledgers.nika` builds every directory with `build`. (issue-1591, #1657)
* `artifact = "rust-library"` has its own path in `project_drive_drive_to` (`crate_build`): a subcommand added there needs both the program case and the library case probed. (issue-1657)

### Scripts

Paths: `scripts/`, `drive.nika`, `agent.nika`, `badges`, `pre-push`

* **Forms** (p1): text to integer `text::parse_i64(s)`; arguments `for a in
  cli::args() { args.push(a) }`; a text is cut only at a position `find`
  returned (NK1214), else `strip_suffix(..) ?? ""`.
* **A change to `scripts/X.nika` runs the tests that name it**: `grep -l X
  tests/compiler/tests` (7970db8).
* **A script cannot `use` another script or find its own binary**; a test
  cannot set `RUSTFLAGS` or see git state: check by hand runs, say so under
  `Deviation`.
* A `sync` tools function cannot call `fs::write` (NK2202); write through the host, e.g. `host.write_if_changed`. (issue-1582)
* **A new `nikaia` flag is declared in clap in `crates/nikaia/src/main.rs` (Rust)**: the command line cannot be Nikaia-only, so a flag needs that Rust plumbing even under *no new Rust*. A development aid read deep inside is kept in a static `OnceLock` set there, not passed through every caller. (issue-1495)
* **`pre-push` fails with *out of date: self-hosted.svg*** when a `.nika`
  changed the self-hosting share: regenerate it from what you commit, `git
  checkout-index -a --prefix=target/badge/`, run `scripts/badges.py` in that
  copy, copy its `assets/badges/self-hosted.svg` back and list it under
  `Files:`.
* A script test that compares a driver's `--dry-run` text fails when `scripts/*.nika` wording changes; bless it from the project dir (`nikaia test --filter NAME --bless` in `tests/compiler`) and review the diff. (qa-task)
* Two `qa-input.txt` files exist (`docs/briefs/` and `docs/briefs/views/`); read the one in the lane directory your task names. (qa-task)
* `scripts/where.nika -- NKnnnn` can run past the 120 s tool timeout; grep the `NKnnnn` in `crates/` and the spec row instead. (issue-1155)
* `pre-push` "self-hosted.svg out of date" can persist even when `badges.py` on the staged tree gives a byte-identical badge; compare with `cmp` and report it, don't chase it. (issue-1261)
* A driver step is tested by running `{repo}/scripts/drive.nika -- --SUBCOMMAND` with `dir:` set to a temp git repo, as in `tests/compiler/tests/drive_relower.nika`; a step with no subcommand gets one added. (issue-1348)
* `badges.py`'s self-hosted share reads `crates/nikaia/src/**/*.rs` plus the `COMPILER_NIKA` tools files, so a temp repo for it needs those paths from `git archive HEAD`, without `lowered/`. (issue-1348)
* A commit that adds Rust makes `assets/badges/self-hosted.svg` stale at HEAD, so pre-push `badges`/`version` and `tests/compiler/pre_push_staged_only` turn red until the supervisor runs `badges.py`. (issue-1582)
* The ADR that ADR-270 replaced is archived: cite ADR-270 (D2→D4, D6→D22) and never name the archived number, not even in a warning, or pre-push's adr-refs check is red. (issue-1591, #1662)
* A test of a driver step that builds a compiler needs a stub `cargo` on PATH whose built "compiler" bakes in the source it was built from; with a BUILD of `true` the old code finds no compiler and the test passes there for nothing. (issue-1665)

### Prover

Paths: `sv_`, `logic_`, `prover`, `prove.rs`, `bounds_`, `--bounds`, `--asserts`, `solver`

* **An index read out of an inner list of a field** (`a = self.col[c][i]`, then `self.atoms[a]`) stays checked whatever length relation the lists have, until ADR-306 D28 is built; a local list of lists is proved already (D7). Sort such a site under D28, not under #385 (that is values computed from the same table). (issue-1470)
* **A field rule holds only if every `ref mut self` method of the type keeps it** (checked at each `return`; `most_facts()` is 48, `rules_rounds()` is 8). One method that loses it (as `score` did for `sv_walk`) drops it for the whole type. So a probe for a field rule is reduced from the real type: the target function plus every `ref mut self` method, with stub types. A synthetic probe can pass while the real code stays checked. (issue-1470)
* **The walk keeps only the newest 48 facts (`most_facts()`) and drops the oldest without a word.** Re-pushing many facts after each loop or branch evicts the length facts a proof needs; add rule facts only where a goal reads them (`walk.rule_facts`, added in `bounding_known`). `shape.bound` includes the parameters, so "bound by the body" must also exclude `args`. (issue-1470)
* **Expressions in the bounds walk carry no byte span** (`node_key` is a node id): a goal's line is its statement's first byte (`walk.at`, set in the statement loop), as in the `--bounds` report; `--bounds-trace LINE` shows the goals of statements starting on that line. (issue-1495)
* **`rules_give` pushes about 3 facts per length rule**, so a type with many equal-length fields fills the 48-fact cap and later goals fail without a word (ADR-306 D30, #1500, moves the cap to the question). To see why a rule is dropped or a goal fails, put an env-gated `eprintln!` into the lowered `.rs` (`walk.rule_failed.insert` in `bounds_rules.rs`, the goal in `bounds_walk.rs`), rebuild without `lower-std`, then restore the file by name; it finds in one build what guessing took 15 turns for. (issue-1470)
* Every turn fact of a loop names that loop's `#turn` variable, so "facts connected to the goal" through shared names takes in every list the loop grows; weight a name by how many facts name it. (issue-1500)
* `rules_settle` recomputes `rules_holding` (the fixpoint over every `ref mut self` method of the type) for every body it walks, so a change that adds field-rule candidates multiplies the cost of a type like sv_walk's; only `bounds_fields` (a plain ask, no ledger) may cache it, since `ask_in` writes each proof into `nikaia.proofs` per function. (issue-1470)
* **The walk gets no fact from `let end = a.min(n)`** (`end <= n`), so `s.bytes[0..<end]` stays checked whatever `n` is, until #1514. (issue-1496)
* `lower-std --bounds` writes nothing and reads no cache, so the old and new binaries can measure the same tree in parallel and be diffed site by site. (issue-1500)
* Code that runs for every question of the bounds walk must not loop over string-keyed maps; give the names integer ids first, or `lower-std` grows from minutes to tens of minutes. (issue-1500)
* **Before adding debug prints to `bounds_*.nika`, run `nikaia check --bounds --bounds-trace LINE F.nika`** (`lower-std --bounds --bounds-trace fixed.nika:95` for a toolchain file: the bare name, matched against the end of the path; one that matches no file prints nothing, #1780): for each goal of a statement starting on that line it prints the goal, every fact it was asked with (rule facts and ledger facts included), how many the 48-fact cap evicted, and the type's field rules. (issue-1495)
* **A rule's fact over every element of a list is guarded by the list's emptiness**: `bounding_each_guarded` turns it into `len(xs) <= 0 || fact`, and `bounding_each_present` gives `len(xs) >= 1` where an element is actually read. A new kind of rule (as D28's `inner`) states its element facts through these, or it proves anything under an empty list. (issue-1483)
* **A list field that a free function such as `main` writes is dropped from the rule candidates**, so a rule test fills its lists only through `ref mut self` methods. (issue-1483)
* **Derived entry claims (ADR-391) key on the first non-`self` parameter**, so a method that takes only `self` gets none. (issue-1483)
* **A trace probe's goal must need the fact under test**: `if ks.len() >= 1 { let y = ks[0] }` proves from its own guard and shows nothing about a join or a rule; ask a goal over another list that only the tested fact reaches (`if ks.len() == 2 { let y = r.ks[1] }` for a kept `ks.len() == r.ks.len()`). (issue-1702, #1889)
* **Derived claims are proposed in `derived_claims` (`bounds_derived.nika`) and only stated at the call by `bounding_derived_side` (`bounds_walk.nika`)**: a claim never proposed shows no row in `--bounds`, not even an unproved one. (issue-1571)
* **A `let` whose value the walk cannot write as a linear term or a bounded range is a number only through the checker's `whole_fields`** (calls, fields, and `x ?? d`); a value outside it drops every guard fact on the name. (issue-1571)
* **`nikaia check --bounds FILE`** checks a file alone; a tools file is
  measured with `lower-std --bounds --asserts > R` and `nikaia run scripts/proved.nika -- R`,
  before and after on the same tree. Where the walk stops:
  `docs/formal-verification.md` §2.
  The two can differ: `check --bounds FILE` proved a site D23 reaches that
  `lower-std --bounds` did not (#1465); a figure is claimed only from
  `lower-std`. (issue-1414)
* **`--asserts` lists `assert` claims only (ADR-361 D7)**; an index or
  counter the walk leaves checked, with the loop its reason names, shows in
  `--bounds`. An issue that asks for it under `--asserts` means `--bounds`. (issue-1634)
* **One `nikaia check --asserts` (or `--bounds`) measures a whole package**:
  it reads every file of the package of the FILE it is given, and its report
  covers each of them under a `--- src/F.nika` line (`project_check_explain`).
  So "how many asserts does this change newly refuse in `packages/P`" is one
  call per binary, `check --asserts packages/P/src/main.nika > R`, diffed
  between the old and new binary; never a loop over the files, which checks
  the whole package once per file (#2232: 35 files, about 20 minutes).
* **A `--bounds` reason's wording is quoted verbatim by the `.bounds` files**
  (23 for the loop reasons): before changing one, `grep -l 'PHRASE'
  tests/compiler/tests/*.bounds` and bless them all in one `nikaia test` run
  (one `--filter` per test). (issue-1634)
* **A `for` binding is a number of the walk** (`CheckFacts.whole_loops`):
  check it before blaming the prover when a guard on `for k in ks` is lost.
  (issue-1414)
* **A test's `.bounds` keys sites by line**: an edited comment keeps its line
  count, or bless again. A precondition on a parameter's length is carried to
  the function's start (ADR-269 D20); a call is carried back only from the top
  of a body. After a precondition change run the `grep -rln 'assert('
  crates/nikaia/tests/it/` modules, not the whole `it` binary.
* Nikaia has no `i8`; widening is `i64::from`. Valgrind needs `x86-64-v3`.
* **A program written for speed (a `benches` row) is checked with `nikaia
  check --bounds FILE` until every index proves, before it is counted**: one
  checked index in the inner loop costs more than the lanes save (#1806's
  first shapes counted 938 and 451 instructions a step against the plain
  program's 326), and a harness run takes about 4 minutes. What proves:
  `Array[T, N]` fields, not `Vec`; indexes written as linear terms of `for`
  bindings (`4 * i + j - 1`); a counter that grows in a loop proves only by
  ADR-361 D13's turn bound, so read its `--bounds` row before relying on it.
  (issue-1806, #1835)
* **Bless bounds expectations**: `nikaia test --bless` writes `NAME.bounds` and `NAME.loops.bounds` only when they exist, a new test's included: create an empty `NAME.bounds` first. (issue-1397, issue-1414)
* **A `.bounds` test checks a private function's body only through its callers**: private functions get derived preconditions (ADR-391 D1), so make the tested functions `pub` for the expectation to read the body. (issue-1414)
* **A length in a solver fact goes through `bounding_length_named`** (typed `i64`, capped at 2^60 when its elements take space, ADR-385 D1); a bare variable as a length proves nothing past `+ 1`. (issue-1465)
* **`Expr::Call` covers constructor paths** (`collections::BTreeMap()`): a walk arm on
  `Call` names its callee, or it poisons map roots; only the `prove_` tests showed it. (issue-1453)
* **D21 takes a list as an end only when its `.len()` already appears in a fact**: a change
  that forgets lengths restates one. Roots come only from `read_let` (`bounds_shape.nika`);
  `whole_loops` is filled by the checker (`check_core_stmt.nika`), not by the walk. (issue-1453)
* **A proof that holds under `check` but not in `lower-std`**: first suspect ADR-391 D1 (a callee also called from another file of the package takes no bound from its callers). Rebuild it as a two-file package, the callee called from the second file, before suspecting the solver budget or the toolchain; a whole-package scratch run takes minutes, the two-file one under a second. (issue-1465)
* **`f.rank[f.find(c)]` (a `ref mut` method call in an index of the same value) passes the checker and fails rustc (E0502)**: a defect, #1445; until it lands, a test probe binds the call to a `let`, and no code under `tools/` relies on the form. (issue-1414)
* `BodyShape.bound` holds the parameters too: "the body never binds it again" is `!twice.contains(name)`. `BoundsChanged.lengths` counts growth as well as shrinking; "never shrinks" reads `falls`. (issue-1397, issue-1414)
* The tools package is a library the Rust crate imports and writes (`emit/host.rs`), so under ADR-306 D22 its `pub` fields of `pub` types are never seen-only-by-Nikaia. Element-bound machinery: `fields_seen`, `fields_declared`, `bounds_fields`. (issue-1414)
* Whether a body keeps a list's length is answered by `changes_length_in` plus `rebinds` (`bounds_body.nika`, `bounds_basic.nika`); parameters are registered in `prover_walk.nika`'s function walk, near `has_a_length`. (issue-1375)
* **Generated crates are re-lowered by the rustc wrapper** (`project_drive.nika`, host `lower` in `crates/nikaia/src/project.rs`) from `Settings::from_env()`: an emit setting changed only in `lower_member` never reaches `gen/`; a per-crate choice passes through the wrapper's `lower` (as `program` does). (issue-1543)
* **A program test links its package as a `[lib]`** next to the program, so a once-per-binary item (`#[global_allocator]`, a `main` hook) is emitted only for `--crate-type bin`. (issue-1543)
* **A runtime panic outside the program's line table prints thread name, file line and backtrace**, which differ between the two settings: unwind with `resume_unwind` (as `OverBudget` does) or expect only stdout. (issue-1543)
* **`std.contracts`' `keeps` column decides whether a std method takes its argument by value**: without it a parameter handed there is lent as `&T` and rustc refuses (NK1343). (issue-1544)
* **A generic `T` handed to a std method whose Rust impl has a bound** (`Sender::send` DeepSize, `HashSet::insert` Hash + Eq) fails rustc, because ledger signatures carry no bound (ADR-024 D9); decided in #1559 (implied DeepSize, `[T: Eq]` lowers to `Eq + Hash`). (issue-1544)
* **A channel probe needs `use std::channel`**; `rx.recv() ?? x` inline gives NK1342: bind first, `let v: T? = rx.recv()`. (issue-1544)
* **`lower-std` runs the bounds walk**, so a slower walk slows every `lower-std` (one ran past 20 minutes and was killed): while you change the walk, lower with the binary before your change and measure with yours once. (issue-1470)
* **After a change to any `bounds_*.nika`, each `lower-std` re-proves std with the new walk: 10-13 minutes, not the 40 s of `builder.md`.** Plan one such cycle per hour; iterate with `check --bounds` on a probe reduced from the real site, and find a slowdown with a few `gdb` stack samples (a minute), not callgrind (an hour). (issue-1470)
* **The checker passing a new expression shape does not mean rustc takes its lowering**: before a re-proving cycle, put each new shape (with the same types: a map read joined in a `match` arm, an `??` fallback) in a scratch program and `nikaia run` it with the old binary; it reaches rustc in a second, where `check` and the cycle's first `lower-std` pass it. `match lit { true => 3, else => m.get(k) }` over a `BTreeMap[String, i64]` checks clean and is rustc NK1343 (`Option<&i64>`, #2170): #2161's run lost two 10-minute cycles to that and to `m[k]`. (#2171)
* **Another agent's uncommitted `bounds_*.nika` makes your cycle re-prove std too** (`lower-std` reads the working tree): run `git status --short crates/nikaia-std/src/compiler/bounds_*.nika` before the first cycle; with one modified there, plan 10 minutes per `lower-std`, run it in the background, and need fewer cycles; do not touch their file. (issue-1731, #1745)
* **A change to the bounds walk that asks new questions makes committed `nikaia.proofs` stale** (the examples', a package's): `build --locked` then fails with "N questions were searched", but only on a clean target, since a warm cache hides it. The driver refreshes them before each push and after merging `origin/develop` (`drive_ledgers`), and CI fails a stale one; by hand, `nikaia run scripts/ledgers.nika -- --check` finds them and `-- --refresh` rewrites them, so commit the added entries with the change. (qa, 3c055bf5; #1591)
* `lower-std --bounds` prints only verdicts; a site's goal and facts come from `lower-std --bounds --bounds-trace FILE.nika:LINE` (the bare file name, `fixed.nika:95`: it matches the end of the path, so the old `tools/fixed.nika:95` matches nothing since the move to `compiler/`, and a trace that matches no file prints nothing, no error), which is cheaper than re-running the whole walk to re-measure one site. Name the `.nika` line, never the lowered `.rs` one. (issue-1496, #1780)
* A new `sv_*.nika` goes into `packages/logic/src/` (the package `logic`, ADR-270 D26, #2020); `scripts/solver.nika` keys and builds the rod from the whole package, so no file list names it. (issue-1339, issue-2020)
* `test` is a reserved word, and the error shows only once the concatenated solver program is built. (issue-1339)
* The prover (`prover_*.nika`) reads only the syntax tree; a checker fact reaches it only by adding a field to `ProveComptimes`, filled in `comptimes_of` (`prove.rs`, called from `check/host.rs`) and copied into `ProverState` by `prove_state`. (issue-1358)
* the bounds walk's loop candidates never read a `for`'s binding, and the prover proposes only for a `while`; past a `for`, bound a counter by its turns (D13), not by an invariant over the counter. (issue-1383)

### Shared checkout

Paths: `Partial:`, `autostash`, `MERGE_HEAD`, `sweep`, `two commits`

* **An uncommitted edit of yours gone from the tree mid-cycle is in the driver's merge** (`git merge --no-commit --autostash`, by design, #1333): `.git/MERGE_HEAD` exists and `git stash list` shows the autostash. Never type the edit again (the autostash pop then conflicts); wait in one call for the merge to end (Monitor until `.git/MERGE_HEAD` is gone), check the edit is back, then run the cycle again (#1830). Meanwhile `commit.nika` refuses (`a merge is in progress`, #1901). An edit still missing after the merge ended is in an autostash git could not put back: `git stash list`, then `git stash show -p stash@{N}` (#1901). Where the merge also changed a lowered file of yours, the driver gives that file HEAD's copy and prints `MERGE autostash conflicted in ...`: run `lower-std` again; a conflict in a source escalates, and the stash keeps your side (#1862).
* A build that shows none of your edits may have run during a driver rebase: check `git status` and your file, then rebuild; name others' `UU` files under `Needs:` (BT, b1043).
* **A file another agent also edits** goes under the report's `Partial:`,
  not `Files:`: stage only your hunks (`git apply --cached`), and
  `commit.nika` commits what is staged of it without `git add` (#1704).
  When their edit is in the same compiler `.nika` as yours, its lowered
  `.rs` mixes both and cannot be split by hunks (the `__nikaia_tupleN`
  names are byte offsets): lower HEAD plus your hunks once in a copy and
  stage its blobs (`scripts/lower_mine.nika` does the copy and the
  lowering for whole files, #1875). `git archive HEAD | tar -x -C $S/head`, `git diff
  F.nika > $S/p.diff`, drop their hunks, `git -C $S/head apply`, then
  `NIKAIA_LOWER_CACHE_SALT=$RANDOM target/fast/nikaia lower-std --sysroot
  $S/head/crates`; for the `.nika`, its `lowered/*.rs` and `lowered.rs`:
  `git update-index --cacheinfo 100644,$(git hash-object -w
  $S/head/PATH),PATH`, list them under `Partial:` (issue-1839, #1858).
* **Your change is a sweep** (it rewrites many files other lanes edit, e.g.
  a visibility pass over every compiler `.nika`): make it your run's last
  step and commit it in one go (edit, `lower-std`, build, commit), never left
  uncommitted across probes or test runs; every other run meanwhile has to
  stage its own files hunk by hunk around it (#1808, #1850).
* **A defect fix and a step in the same file are two commits**, but
  `commit.nika` stages whole `Files:` (or list it under `Partial:` with only
  the fix hunks staged): commit the fix (the `.nika` and its
  lowered `.rs`) before starting the step in that file. Met late, put HEAD's
  version plus the fix hunks in place, commit, then restore the step
  (issue-1470, issue-1691). Two fixes in different `.nika` lowered in one
  cycle are two commits too: the first lists its `.nika`, its lowered
  `.rs` and `lowered.rs` (`commit.nika` writes the digest of what is
  staged, so the tree's `lowered.rs` still differs for the second); a
  lowered `.rs` both fixes move is staged for the first from `lower_mine.nika
  -- FIRST.nika` (`git update-index --cacheinfo` of its blob in
  `target/lower-mine`, under `Partial:`, as for a file another agent also
  edits, above) (#1883).
* `git merge --no-commit --autostash` keeps the other agents' changes stashed until the merge is committed, so regenerate inside the uncommitted merge, never after it. (issue-1333)
* During a merge, `git status --porcelain` also lists the files the merge staged; use `git diff --name-only` (worktree against index) to see only what a regeneration changed. (issue-1333)

### Measuring

Paths: `callgrind`, `measure`, `instructions`, `cost`, `bench`, `valgrind`

* **A Nikaia program that reads standard input hangs under callgrind, silently, unless it runs with `io-method = "blocking"`** (#1874's `io_uring_enter` wait, `docs/measuring.md`): a game-bench binary counted by hand, outside `benches/game/game.nika`, runs as `NIKAIA_RUNTIME_CONFIG=target/game-bench/io-blocking.toml valgrind --tool=callgrind BIN < target/game-bench/out/NAME.input` (the file `game.nika`'s `nikaia_env` writes; elsewhere a file holding that one line), and with `timeout -s KILL`, since a plain `timeout` does not end the hang. (#2072, #2361)
* A whole `lower-std` under callgrind takes hours; an A/B of it is user time with the bash `time` keyword, two interleaved rounds, twin binaries built from the same source. Uncached: set `NIKAIA_LOWER_CACHE_SALT` and pass `--sysroot` a copy laid out as `X/crates` with `X/packages` beside it. To profile one phase, run it with `NIKAIA_LOWER_TIMES=1`: each phase and ledger pass prints a `lower-std~ NAME ended` line the moment it ends, so `callgrind_control -d` then dumps the phase so far (`--dump-after=` misses them, the phase functions are inlined). A criterion in instructions over a whole `lower-std` (ADR-361 D15) is counted on its parts as `docs/measuring.md` says. (issue-1473, #1979, #1980)
* The `lower-std` cache key includes the `Build` settings, so an A/B of two settings needs a warm-up run for each binary; the first run with a fresh salt is the cold figure. (issue-1473)
* **A Nikaia program counted by hand under callgrind needs `NIKAIA_RUNTIME_CONFIG`** naming a file with `io-method = "blocking"` (`docs/measuring.md`): valgrind 3.22 runs the executor's `io_uring_enter` wait without blocking, so a program reading standard input (a `benches/game` binary rerun outside `game.nika`) waits for ever, silently. `game.nika` writes that file as `target/game-bench/io-blocking.toml`: `NIKAIA_RUNTIME_CONFIG=target/game-bench/io-blocking.toml valgrind --tool=callgrind ... BIN < target/game-bench/out/NAME.input`. Bound every such run with `timeout -s KILL` (a plain `timeout` does not end it). (#1874, #2361)

### Wip branches and drafts

Paths: `wip/`, `draft`

* A wip branch whose single commit sits on an ancestor of HEAD is applied with `git diff HEAD...wip/X | git apply`, after `git diff --stat BASE HEAD` shows HEAD has not touched its files; after `lower-std`, an empty `git diff --stat wip/X -- crates/nikaia-std/src/compiler/lowered` shows its lowered files are current. (issue-1470)
* The fixpoint check after applying a wip is `git diff --stat wip/X -- crates/nikaia-std/src/compiler/lowered` after `lower-std`: empty means the lowered files committed on the wip are current. (issue-1470)
* When a wip draft was built before your run, `target/fast/nikaia` already holds it; build the before binary by checking out HEAD's version of only your own files, building, then `git apply` your saved patch. (issue-1473)

## Moving Rust into Nikaia (self-hosting, ADR-294, #125)

### Slices and patterns

Paths: `slice`, `move`, `crates/nikaia/src/`

* **A move is translated by hand from the Rust** (owner, 2026-11-01).
  Iterator chains are the house style (owner, 2026-10-27).
* **Pick the slice by the call graph, leaves first.** The `Checker` and the
  emitter's state become Nikaia structs (#558); a method moves into an `impl`
  in Nikaia once everything it calls has moved; the recursive core (`value_of`,
  `stmt`, `call`; `expr_as_written`, `a_stmt`) moves in one step. `grep
  'Finding {'` and `format!(` find the cheap message-building first. A walk
  that cannot move yet takes a callback (`fn(ref Expr) -> i64 sync`, see
  `bounds_basic.nika`).
* **Copy a pattern**: `libraries.nika` + `libraries.rs`, `assets.nika`,
  `parse_numbers.nika` (callbacks), `interpreter.nika`. The Rust module stays as
  the adapter that calls `nikaia_std::compiler::<m>::f`.
* **One namespace for all `tools/*.nika`**: prefix type and function names with
  the module; calls to other tools files are unqualified.
* **Never compare a contract by address** (`std::ptr::eq`). Destructure map
  loops (`for (name, value) in map`, `for (name, _) in map`), not
  `entry.0`/`entry.1`.
* **Delete the dead locals** a replaced block leaves, and re-grep function names
  instead of trusting line numbers from an earlier listing.
* the old binary lowers `for x in xs.f[a..]` wrong (NK1343, `expected Symbol, found &Symbol`); after the checker fix in fb2698e9, the first build of a tool that uses it needs the index-skip form, then a switch back. (issue-1383)
* `for rule in found` lends each element from the named local, so `held.push(rule)` in the loop is refused (NK1102, a borrowed value moved); write `held.extend(found)`. (issue-1470)

### Syntax gaps of tools/*.nika

Paths: `tools/`, `crates/nikaia-std/src/compiler/`

* `grammar` is a reserved word: a binding named `grammar` in `tools/*.nika` is refused by `lower-std`. (issue-1431)

* `tools/*.nika` has no `loop`; write `while true`. (issue-1266)
* `for x in E ?? return null {` does not parse; bind `E` with `let` first. (issue-1266)
* `for (i, mut x) in v.iter_mut().enumerate()` is refused (`mut` in a tuple pattern); use one read-only enumerate loop and a separate `for mut x in v` loop. (issue-1266)
* `a..b` includes `b` in Nikaia; write `a..<b` for the half-open range. (issue-1218)
* A trait method's signature ends at the line end; a `;` after it is a parse error ("Expected `}`"). (issue-1224)
* A `match`'s catch-all arm is `else => ...`; `_ =>` is refused. (issue-1352)
* A text position taken from `match s.find(..) { at => .. }` loses its origin (NK1214); write `let at = s.find(..) ?? return null` in a helper. (issue-1339)
* `i64` has no `checked_div_euclid`/`checked_rem_euclid`; guard `MIN / -1` and use `div_euclid`/`rem_euclid`. (issue-1339)
* A trailing `;` does not drop a value and `let _ = x` is NK1144, so a value can't be discarded explicitly; a body's last value is dropped only where the function type hands back nothing. (issue-1321)
* `let mut s = ""` and then assigning an owned `String` is NK1105; start it as `String()`. (issue-539)
* A `for` over a list literal of strings binds views, so pushing the loop variable into a struct field needs `.clone()` (NK1106). (issue-1404)
* A `T?` is matched with `x => …` and `null => …`: `if let` and `Some(x)` are refused (NK2704), so write probes that way. (issue-1417)
* A `??` whose left side reads a map value (`m[k] ?? []`) is NK1185 in tools code; write `m[k]?.clone() ?? []` behind a `contains_key` check. (issue-1485)

Classes (CLAUDE.md, *A missing form stops the work*): **[design]** holds,
build that way; **[gap #N]** stops the slice (`Result: blocked`); **[defect #N]**:
fix it or wait; **[works]**: use it, do not copy an old way around.

* [design: ADR-382 D6] no `if let`: `x ?? default`, `match`, or `starts_with`
  then `strip_prefix(..) ?? ""`;
* [design: ADR-094 D4, ADR-231] no `*x`; loop variables over a `ref Vec` are values;
* [design: Part I 2.5, ADR-282] `.to_owned()` is `NK1189`: write `.clone()`;
* [design: ADR-282 D2; transition #654] text is `String`; never `ref` on a
  number, `bool` or a data-less enum (`NK1257`). Until #654 step 0, `ref
  String` only in: a field; a type argument; an option after `;`; a function's
  or rule's result; a parameter the result points into; a parameter used as a
  key of a map keyed by `ref String`. A parameter only read is `String`;
* [design: ADR-293 D19] `opt ?? ""` moves the option: `(opt ?? "").clone()`
  for a second use; a method on a `String?` is `opt?.len() ?? 0`; a `T?` local
  needs `?.clone()` (NK1125);
* [design: Part I 2.5] f-string braces are `{{` `}}`;
* [design: ADR-288] a function parameter is `fn(ref X) -> T sync`; a `Span` by
  reference is copied with `s.clone()`;
* [design: one namespace] `view_of` is `signature::view_of`: a false `NK1104`
  (*returns `ref String`, but declared `ref String`*) means the wrong one;
* [design: ADR-352 D5] `Some(..)`/`None` are no patterns of a `T?`: bind `how
  ?? ...` first; a `String?` field is matched as it is (`null => .., x => ..`);
* [works] a `ref String?` parameter takes a bare `String` (`ref String?` is
  `NK1261`, write `String?`);
* [works: ADR-331 D5] a struct field may have a default (`pub edit: Edit? =
  null,`), and a literal in any file of the package leaves it out: a field
  added to a struct with many literals (`CheckFinding`) gets one, not an
  `edit: null` in each; literals in hand-written Rust still name it;
* [works, #1133] `mut` on one name of a tuple marks that name alone:
  `let (a, mut b) = …`; `for (k, mut v) in m` lowers to `m.iter_mut()`,
  `for (_, mut v) in m` to `m.values_mut()`; only a lambda's tuple parameter
  is still `NK1307`;
* [design: Part I 3.3, 3.4] `a..b` includes `b`, half-open is `a..<b`; the
  catch-all arm is `else =>`;
* collections: [works] `m.iter()`, `for (k, v) in m`, `a.iter().zip(b.iter())`;
  in-place change of a map value `let mut c = m[k] ?? continue` (#952); [gap
  #950] no `BTreeSet::extend`, no `BTreeMap::entry`; [gap #949] no
  `mem::take`; [design: Part I 2.6] a text is cut only where it handed out a
  position (`NK1214`): `hex.scalars.take(16).collect()`; [design: Part III]
  `fs::read` is async and gives `Bytes`; `read_dir` gives sorted names.

### Lowered types and host traits

Paths: `host.rs`, `host trait`, `lowered/`, `crates/nikaia/src/`

* `String` is `&str` where a view suffices and `String` where owned (ADR-282
  D2), `ref T` is `&T`, `String?` is `Option<String>`. Most Rust errors after a
  move are `&` against owned. Check that `Display` equals `Ty::text()` before
  replacing `{ty}`.
* A host method that `throws` lowers to `Result<T, Box<dyn std::error::Error>>`;
  keep an `anyhow` refusal in a `RefCell` of the host to hand it back. A
  body-less trait method's text parameter is written `ref String`. Host state
  is a `RefCell` field; the Nikaia side names units by `i64` index (M3b).
* **`Vec` in the Rust host is Nikaia's `List`**: most of `crates/nikaia/src`
  (`project.rs`, `emit/`, `sysroot.rs`, ...) imports `nikaia_std::block::names::{Vec, vec, ...}`,
  and `block.rs` has `pub use super::List as Vec`. A plain Rust buffer there
  (bytes read from a pipe, `Command` args, a value handed to `std` or libc
  code) is written `std::vec::Vec` / `std::vec::Vec::new()`, as `project.rs`
  already does; `cargo check --release -p nikaia --message-format short`
  catches the slip before a build. (issue-1919, #1923)
