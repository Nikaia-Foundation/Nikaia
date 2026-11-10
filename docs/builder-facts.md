# Builder facts

Fixed facts a builder would otherwise look up itself. `scripts/agent.nika`
puts *Core* into every builder's prompt, and after it the sections the brief
names (`Facts: Probes, Grammar`; a `##` heading brings its `###` sections
along), taken from `docs/builder-facts-lookup.md`. That file is not read
whole: `nikaia run scripts/syntax.nika -- KEYWORD` prints its lines on a topic
together with the specification section and an example test.

## Core

### Where things live

| What | Where |
| :--- | :--- |
| Checker, moved parts | `crates/nikaia-std/src/tools/check_*.nika`; Rust adapter in `crates/nikaia/src/check/` |
| Emitter | `crates/nikaia/src/emit/mod.rs`, `emit/host.rs`; moved parts `tools/emit_*.nika` |
| `std` methods by receiver | `std_receivers` in `tools/check_core_method.nika` |
| Generated Rust of a tools file | `tools/lowered/<file>.rs`, index `tools/lowered.rs` (never edit, `lower-std` writes them) |
| Program tests (new tests) | `tests/compiler/tests/NAME.nika` + `.stdout` / `.stderr` |
| Rust tests by NK code | `grep -rln NK1234 crates/nikaia/tests/it/` |
| Specification | `docs/specification/10-nikaia-light.md`, `20-nikaia-advance.md`, `30-nikaia-tooling.md` (code table) |
| A name, Rust or Nikaia | `nikaia run scripts/where.nika -- NAME...`: definitions with file:line and the files that call them; for an `NKnnnn` the tests and the spec row (a field is not found: grep it); `-- --outline FILE` lists each item of a file with its line range |
| A snippet through the built compiler | `nikaia run scripts/probe.nika -- 'let x = 1\nprintln(x)'` (`--check`, `--file F.nika`): runs it with `target/fast/nikaia`, prints at most 60 lines and `exit N`; a probe for a report is the snippet and these lines |
| How a form is written | `nikaia run scripts/syntax.nika -- KEYWORD`: the specification section, an example test and the facts lines on it (e.g. `slice`, `view`, `throws`) |

### Every run

* **ADRs live in `docs/specification/adr/`**; search them with the Grep tool, since a `grep -r` from the root runs into `target/`. A script test run by hand resolves a relative `target/fast/nikaia` under its tmp dir: run a copy with the absolute path. (issue-1397, issue-1414)
* **Copy `target/fast/nikaia` to your scratchpad before your first build**: it
  is the before binary for the fails-before run, and the binary long test runs
  use, since another agent may rebuild `target/fast/nikaia` mid-run (P5a, j5).
  Keep the copy in its own directory and name it exactly `nikaia`
  (`$S/old/nikaia`): script tests check the basename and otherwise fall back
  to `target/fast/nikaia` (b42). `target/release/nikaia` is older than the
  views syntax and cannot serve.
  The copy may predate HEAD (another session built it from another tree,
  and its mtime is when you copied it): when a fails-before result surprises,
  build the before binary from HEAD's own sources first; a stale copy cost
  a run 30 turns chasing a regression HEAD never had. (issue-1483)
* A rebuild while `nikaia test` runs fails tests at random: rerun the failed
  ones alone before calling them failures. (issue-1414)
* **The checkout is shared.** A file another agent has modified: stage only
  your hunks (`git diff FILE > $S/p.diff`, delete theirs from the copy, `git
  apply --cached`, check with `git diff --cached FILE`; `git add -p` is
  refused here). A build that shows none of your edits may have run during a
  driver rebase: check `git status` and your file, then rebuild; never touch
  others' `UU` files, name them under `Needs:` (BT, b1043).
* **Long runs go in the background** with `run_in_background` and `timeout:
  3600000`, and you wait for the wake-up, never a `sleep` or `pgrep` loop. A
  `lower-std` and build cycle takes 10-15 minutes: find every change it needs
  with probes on the old binary first; one cycle less is the largest saving a
  run can make (a9, v6, v7). Put a `&` on one command, never on a chain (P4b).
* **Probes live in the scratchpad**, outside any project: `nikaia check FILE`
  inside `tests/compiler/tests/` reads the sibling files (prover lane).
* **Never commit `nikaia.lock`**; it is git-ignored and changes with each build.
* **Never run `nikaia fmt` on `crates/nikaia-std/src/tools/*.nika`**: it
  reindents the whole file (b1012b). Format by hand there.
* **`pre-push` fails with *out of date: self-hosted.svg*** when a `.nika`
  changed the self-hosting share: regenerate it from what you commit, `git
  checkout-index -a --prefix=target/badge/`, run `scripts/badges.py` in that
  copy, copy its `assets/badges/self-hosted.svg` back and list it under
  `Files:`.
* **Builders have no GitHub**: the prompt holds the issues the brief names; a
  missing one goes under `Deviation`. A pre-flight names a commit by subject
  (`git log --grep`), never by hash: rebases change hashes (p9).
* A script test that compares a driver's `--dry-run` text fails when `scripts/*.nika` wording changes; bless it from the project dir (`nikaia test --filter NAME --bless` in `tests/compiler`) and review the diff, since `nikaia run FILE` refuses a non-entry file. (qa-task)
* Two `qa-input.txt` files exist (`docs/briefs/` and `docs/briefs/views/`); read the one in the lane directory your task names. (qa-task)
* Before putting back a "workaround" an issue names, check `git log -S` on that line: a later commit may have removed it on purpose. (issue-1125)
* The tether's keep plan (`tools/buffers.nika`, `is_a_buffer` in `tether.nika`) treats only text and bytes as buffers, so a view of a list or struct local handed back is NK2304 (ADR-283 D9), never NK2303, which is not a refusal. (issue-1155)
* `scripts/where.nika -- NKnnnn` can run past the 120 s tool timeout; grep the `NKnnnn` in `crates/` and the spec row instead. (issue-1155)
* Running a copied test file with `nikaia run` inside `tests/compiler/tests/` is refused as not the entry point; run the fails-before check on the scratchpad copy. (issue-1266)
* After a `.nika` change, the cycle's first `lower-std` must run with the before binary (`$S/old/nikaia`); `target/fast/nikaia` still holds the logic of the previous attempt. (issue-1209)
* An issue's probe may pass on `develop` because a later fix masks it; vary the shape (index, `.iter()`, a second call site of another kind) before judging the defect gone. (issue-1210)
* A `git mv` staged before `commit.nika` makes it refuse (staged and not listed, or `git add` of a missing path); delete files unstaged and list both paths. (issue-1218)
* Before planning, `git log --grep=<issue>`: earlier lane runs may have finished part of the issue under `Part of #N`. (issue-1224)
* A test of "a `keep` parameter keeps it" needs a callee that really keeps (stores or spawns); `fn hold(keep w: W) -> i64 { w.k }` only gets an NK1334 warning and proves nothing. (issue-1225)
* `scripts/pre-push.nika` is red on `develop` (self-hosted badge) whatever is staged; compare with an empty index and never reset to find out. (issue-1277)
* `NIKAIA_LOWER_CACHE_SALT=<fresh>` plus `lower-std --sysroot <copy>` gives an uncached reference run to `diff -r` against for byte-identity, without touching the shared cache. (issue-1277)
* A `let` over a field of a `ref` parameter (`let k = index.keys`) is NK2106 by design; read the field in place at each use. (issue-1279)
* Two `--lib` filters in one `cargo test` call (`emit:: sysroot::`) printed nothing for me; run one filter per call. (issue-1279)
* A rustc "temporary dropped while borrowed" or "mismatched types" on text usually comes from the text-tier pass (`text_tiers.nika`, ADR-282), not the keep code: lower the probe and check whether a `Vec[String]` became `Vec<&str>`. (issue-1287)
* `.to_string()` of text is emitted as the bare receiver (`text_as_is`, ADR-282 D8); copying text out of a view is `.clone()`, and a kept view of a temporary is NK1102 by design. (issue-1287)
* `nikaia test --filter X` prints no `test result` line when nothing matches, so a wait loop that counts result lines never ends. (issue-1287)
* `nikaia check` on a single `crates/nikaia-std/src/tools/*.nika` fails with NK1135 for imported types (`Span`); only the `lower-std` cycle checks a tool file. (issue-1289)
* `nikaia lower F.nika` in the scratchpad writes `F.rs` beside it; read `__nikaia_main` there before guessing at an emitter cause. (issue-1289)
* `scripts/pre-push.nika` checks only what is staged, so run it after `git add` of your files, or it is green on nothing. (issue-1290)
* `cargo test -p nikaia-std --lib emit_core_args::` matches no test, because lowered modules have no unit tests; skip it for a `tools/*.nika` change. (issue-1290)
* Typing a path expression `T::X` (variant, method value, `T::fields`, integer limits) is in `check_core_expr_path` in `tools/check_core_expr.nika`. (issue-1294)
* Nikaia's integer types are only `int`, `i32`, `i64`, `u8`, `u32` and `u64` (Part I 2.2); `integer_named` in `check_names.nika` tests for the fixed widths, and `int` is rewritten to `i64` only in emitted types (`emit_core_types.nika`), not in paths. (issue-1294)
* A lifetime or borrow probe needs a non-`Copy` element type (give the struct a `String` field), or the emitter copies the element and the defect does not show. (issue-1228)
* The cycle's first `lower-std` with `$S/old/nikaia` can fail on files nobody changed; the new binary's `lower-std` is the one that counts, and that failure goes in the report. (issue-1228)
* A `from` fix for text must also be probed with a struct type: the trait lowers only text `from` as a view, so a fix in `keeps.nika` breaks the struct case. (issue-1250)
* A text `+` is a `Binary` node that the emitter recognises by `facts.concatenations.contains(span.start as i64)`, using the operator span, not `node_key`. (issue-1251)
* A `let` bound by `x ?? null` used to lose `x`'s `ref` (it was typed as the `null`); if a `&` appears in front of a `match` in the lowered code, check the checker's result type of the `??` first (`check_core_ops.nika`). (issue-1340)
* `git reset -- <paths>` only unstages, but builder.md forbids every `git reset`; to unstage your own file, use `git restore --staged FILE`. (issue-1340)
* Write a new `tests/compiler` program's `.stderr` with `nikaia test --filter NAME --bless` from `tests/compiler`; `nikaia check` there reads every sibling test. (issue-1352)
* Run `pre-push.nika` with your files staged, then unstage only them before `commit.nika`, which refuses files that are already staged. (issue-1352)
* Copy the old binary to the scratchpad before the first build, so each new test can be shown to fail there without a stash. (issue-1254-r1)
* `nikaia check` on a new test inside `tests/compiler/tests/` reads the sibling files; write a test's `.stderr` with `nikaia test --filter NAME --bless` from `tests/compiler`. (issue-1261)
* `pre-push` "self-hosted.svg out of date" can persist even when `badges.py` on the staged tree gives a byte-identical badge; compare with `cmp` and report it, don't chase it. (issue-1261)
* Nikaia has no `loop` or `|x|` closures and declares mutables with `let mut`; write probes with `while true` and `fn(x) { … }`. (issue-1261)
* a view of a lent parameter handed to an owning `String` result is a refusal with a `.clone()` hint by ADR-282 D8 and views-3 D12/D14, whatever an issue's expected output says. (issue-1263)
* A text view in the checker's `Ty` is `Named { name: "str", view: true }` (printed as `ref String`), and `is_a_view_of_a_string` misses it; match `TEXT_VIEW` too. (issue-1264)
* A refusal of a view handed back where `String` is wanted must exempt static text: ask `text_literal(world, a_string_type(), value, true)`, or `"x"` is refused too. (issue-1264)
* Calling code stored in a struct field with an owned `String` argument (`h.f(g)`) is a rustc error on `develop`; avoid it in probes until it is fixed. (issue-1264)
* Programs build against the workspace `crates/nikaia-std` source, so a fix in a std runtime `.rs` file passes on the old binary too; for fails-before, copy HEAD's version of that file back in for one run, then restore yours. (issue-1268)
* A `let x: Array[T]` (no length) is made a view by `lv_let_views` in `text_tiers.nika`, like fields (`sf_slice_fields`) and results (`hb_fn`); extend those passes rather than the checker. (issue-1270)
* Package-wide facts reach a file's tier pass only through `BesideFields` in `crates/nikaia/src/text_tiers.rs` (built in `beside_fields`, read by `refine_with`); add a field there rather than a new parameter at each caller. (issue-1272)
* A two-file package test puts the struct in `tests/compiler/src/X_beside.nika` and the user in `X_beside_caller.nika`, and the program in `tests/` calls it as `compiler_tests::fn`. (issue-1272)
* `cargo test -p nikaia --lib text_tiers::` runs 0 tests; the tier pass is covered only by tests/compiler programs. (issue-1272)
* The cycle in builder.md writes `target/fast/nikaia lower-std` first, but after a failed attempt that binary holds the bad logic; run `$S/old/nikaia lower-std` first. (issue-1274)
* A new `sv_*.nika` must be added to `solver_engine_files()` and to the hard-coded file lists in ten `tests/compiler/tests/solver_*.nika`. (issue-1339)
* `test` is a reserved word, and the error shows only once the concatenated solver program is built. (issue-1339)
* When the before binary's `lower-std` fails on files nobody changed, put HEAD's version of your own `.rs` files back, build, lower, then restore them and build again. (issue-1350)
* Test programs had only `.stdout`, `.stderr`, `.stops`, `.bounds` and `.loops.bounds` expectations until `.asserts` (#1350); a new report expectation goes through `project_tests_bounds_outcome` and the host's `explained(args)`. (issue-1350)
* A change to the `check --asserts` header breaks `it` tests `assert::asserts_reports_every_claim` and `assume::asserts_lists_every_assume`, which match it exactly. (issue-1350)
* A move probe needs a callee that keeps its argument (`all.push(x)` into a list declared outside the loop), or the checker lends the parameter and no move happens. (issue-1366)
* Nikaia's `a..b` includes `b`; use `0..<n` for n turns. (issue-1366)
* After an interrupted build, `cargo test --test it` can fail at link time with "undefined hidden symbol"; delete only `target/fast/deps/it-*` and `target/fast/incremental/it-*`, then rebuild. (issue-1366)
* NK2105 for loops is decided in `hands_over_path_of` and `CheckRepeats` (`check_takes.nika`, `check_state.nika`); `revived` there also drives narrowing (ADR-382), so a new exemption needs its own set. (issue-1366)
* `a..b` is inclusive in Nikaia (lowered `0..=b`); write `a..<b` in probes for a half-open slice. (issue-1276)
* The prover (`prover_*.nika`) reads only the syntax tree; a checker fact reaches it only by adding a field to `ProveComptimes`, filled in `comptimes_of` (`prove.rs`, called from `check/host.rs`) and copied into `ProverState` by `prove_state`. (issue-1358)
* `cargo test -p nikaia --lib prove::` and `--lib check::` run 0 tests; the prover is covered by the `it` tests `prove::`, `assert::`, `assume::` and by `tests/compiler` programs. (issue-1358)
* `nikaia test --filter NAME --bless` creates no `.stderr` when none exists; `touch NAME.stderr` first, then bless. (issue-1361)
* The LSP test programs in `tests/compiler/tests/` are already unformatted on HEAD; `fmt --check` on them proves nothing about your change. (issue-1361)
* NK1111's doubled-brace and hole warnings are both in `unmarked_hole` (`tools/check_host.nika`), and the brace scanning helpers are in `tools/check_words.nika`. (issue-1361)
* A `let` of a choice mixing owned text and a view is typed `ref String` by the checker but emitted as `EitherText`; a missing `&` at a call is a `lent_args` question in `check_arguments.nika`, not an emitter one. (issue-1310)
* In cargo test output, grep `test result:` with the colon; `^test result` also matches `it` tests named `result_source::…`. (issue-1310)
* `.to_string()` of a text slice is a view in the checker as well (NK1105 when assigned to a `String` local), not only in the emitter. (issue-1310)
* ADRs are in `docs/specification/adr/` (for example `adr-views-3.md`); there is no `docs/adr/`. (issue-1280)
* The checker has two text-view forms, `Named{"str", view:true}` and `Named{"String", view:true}` (the latter from `ref $T` with T=String, e.g. `.iter()` and `or_insert`), and both print as `ref String`. (issue-1316)
* Do not merge the two text-view forms in `ty.nika` `substituted`: the `String`-slot form means `&String`/`&mut String` to the emitter, and writes through `or_insert` break; normalise in the one check that needs it. (issue-1316)
* An NK1103 that says "is `ref String`, but the `let` declares `ref String`" is the two view forms meeting, not a tier question. (issue-1316)
* A `from` fix in a trait must change both sides together: the impl's types in `hb_trait_from` (`text_tiers.nika`) and the trait's in `emit_item_lent_view` (`emit_item.nika`); one alone gives rustc a trait/impl mismatch. (issue-1327)
* `nikaia fmt` or `fmt --check` on a file outside a project formats with tabs, and the committed `tests/compiler` tests fail `--check` too; it proves nothing about a new test. (issue-1327)
* A `Found:` filed from an earlier run: read that run's commit message (`git log --grep=#N`) first; it often holds the cause and a path already dropped. (issue-1327)
* A `tests/compiler/tests/*.nika` program only runs under `nikaia test --project tests/compiler --filter NAME` (`nikaia run` refuses it as not the entry, and its `toolchain()` finds the compiler through its parent process). (issue-539)
* `fs::read` returns `Bytes`, so a `catch { "" }` on it is NK1103; in a test program, read text with `process::run("cat", …).stdout`. (issue-539)
* When `commit.nika` refuses another agent's staged file, commit only yours with `git commit --only -F MSG -- FILES`, which leaves their index entry as it was. (issue-1375)
* A test whose assert is refused (NK1300) does not build, so it cannot sit beside a `.stdout`: put the refused case in its own test with a `.stderr`. (issue-1375)
* When a mass migration such as `print-throws` rewrites test programs, the `accepted_sweep.rs` allow-list hashes change. The sweep's failure log pairs each new refused hash with the old "no longer checked" one, so replace the old hash with the new one. (qa-task)
* `commit.nika` refuses when another agent's file (even an unmerged `UU` one) is staged. `git commit -- PATH` commits only your path, and the report should say so under `Deviation:`. (qa-task)
* the bounds walk's loop candidates never read a `for`'s binding, and the prover proposes only for a `while`; past a `for`, bound a counter by its turns (D13), not by an invariant over the counter. (issue-1383)
* pre-push shows `badges red` / `version red` (self-hosted.svg) on `develop` with nothing staged; it was red before this run, and the badges are the supervisor's. (issue-1383)
* A brief that needs a new NK code without naming a reserved one is blocked at once; search `docs/supervisor.md` and the brief for an NK block before planning. (issue-1318)
* A probe or test program whose `main` calls `print`/`println` needs `fn main() throws` (ADR-392): a probe otherwise stops at NK2605 before the defect shows, and `nikaia test --bless` writes an empty `.stdout` and a stray `.stderr`. (issue-1318, issue-1445, issue-1449, issue-1453)
* `docs/briefs/` is untracked: `Fixes #N` goes on the last code commit, not on a brief or report. (issue-1453)
* A debrief or report an issue cites may be missing from `docs/briefs/`; read `git log --grep='#N'` instead. (issue-1470)
* "file NNN" in a `Found:` of the #1254 runs means `tests/accepted/NNN_*.nika` migrated by dropping `ref` from its parameters; reproduce on that file, not only on the reduced probe. (issue-1334)
* Running two `nikaia test` processes on `tests/compiler` at the same time can make `migrate_views3_reads_package_tests_with_the_package` fail with "cannot read TMP/json/tests"; it also fails alone on develop, so it is not your change. (issue-1334)
* A checker change that makes a tools file fail to check must be lowered with the old binary first (`$OLD lower-std`), or the change never reaches `lowered/`. (issue-1388)
* `container_of` in `tiers.nika` returns null for `ref` types, so the tiers pass does not follow a `ref Vec[...]` parameter, and its `String`s stay `String` below (`lent_whole`). (issue-1449)
* Run the issue's probe on develop first: a probe made only of Copy fields can pass there and hide the defect. (issue-1342)
* The first `lower-std` with the old binary resets lowered files a broken build left behind; after a failed build, restart the cycle from the old binary, not the new one. (issue-1344)
* The checker types `b[a..<c]` on a `Bytes` as an owned `Bytes` (ADR-320 D7), but the emitter writes a `&[u8]`; the fact `bytes_cuts` marks such a cut. (issue-1345)
* `check_crossings_arguments` gets the unbound contract, so `want` there is the type variable `K`; the substituted types are `expected`/`bound` in `check_core_call_on`, and `entry`'s key is `?` (use `bound["K"]`). (issue-1345)
* `nikaia check` accepts `HashMap[String, i64]`'s `insert(5, 7)` (std method arguments are not checked against bound types), so a probe that passes `check` can still fail in rustc; always `run` it. (issue-1345)
* A driver step is tested by running `{repo}/scripts/drive.nika -- --SUBCOMMAND` with `dir:` set to a temp git repo, as in `tests/compiler/tests/drive_relower.nika`; a step with no subcommand gets one added. (issue-1348)
* `badges.py`'s self-hosted share reads `crates/nikaia/src/**/*.rs` plus the `COMPILER_NIKA` tools files, so a temp repo for it needs those paths from `git archive HEAD`, without `lowered/`. (issue-1348)
* An owned `String` in a probe is `let mut g = String()` plus `g.push_str("…")` or a `fn` returning `String`; `.to_string()`/`.clone()` on a literal is NK1301 and `String::new()` is NK1149. (issue-1356)
* Before planning, check whether the issue is closed or marked duplicate, and run its probe on `develop`; a fix under another issue's `Part of` may already have landed. (issue-1356)
* No binary from before the views syntax exists (`target/release/nikaia` is absent); a fails-before run for a fix already on `develop` needs a build of the fixing commit's parent, so report it as not done. (issue-1356)
* Turning a move into a lend in the checker also needs `vc_view_of` (`view_changes.nika`) to see the `let` as a view, or a later write to the source passes the checker and becomes a rustc NK1343 internal error. (issue-1362)
* A `let` bound to a cut (`v[0..]`) or a `std` method result (`s.trim()`) is a view only if `vc_view_of` in `view_changes.nika` sees it; otherwise a later change becomes a rustc NK1343 instead of NK2305. (issue-1363)
* The library ledger has no `borrows` entries for `std` methods (`String::trim` is missing from `described.contracts`), so the view-change walk cannot tell that a method result points into its receiver. (issue-1363)
* The tier pass runs without checker types, so a rule there must not mark lists whose items are already views (split/map) as owned. (issue-1368)
* Changing a help's wording re-records many `.stderr` files and the `lsp_diagnostics*.stdout` files; grep `tests/compiler` for the old phrase before building. (issue-1403)
* A probe of ownership needs a `String` made by a call (`fn make()` with `push_str`), because a literal-filled `String` is tiered to `&str` and hides NK1343. (issue-1404)
* `nikaia test` run with a scratch copy of the binary fails `migrate_views3_reads_package_tests_with_the_package` (No such file); rerun it with `target/fast/nikaia` before calling it a failure. (issue-1460)
* "Copied at a read" is `a_word_by_its_kind` (numbers, bool, char, scalar, payloadless enums, ledger `copies`); `a_copy_by_value` covers only the first four, and a copy-out keyed on it misses `Symbol` and enums. (issue-1460)
* A `let x: V? = m.get(k)` or `m[k]` is typed `ref V?` by #297's `of_a_map` block in `check_core_stmt.nika`; a value-typed read must be exempted there. (issue-1460)
* New test programs go in `tests/compiler/tests/NAME.nika` (with `.stdout` and an empty `.bounds` before `--bless`), not in `tests/compiler/`. (issue-1470)
* A copy of `target/fast/nikaia` is no "before" binary: other sessions rebuild it from other trees, so build the fails-before binary from HEAD's `.nika`. (issue-1483)
* Checker facts (`check_facts.nika`) are pure Nikaia with no Rust mirror: a new field needs only its declaration and its initializer there. (issue-1411)
* A defect that shows only when the call crosses packages is often a type `qualify` (`ty.nika`) leaves unqualified, so a dependency's `H` never matches the caller's `lib::H`; check that before the lend logic. (issue-1417)
* A cross-package probe or test copies the `mktemp` two-package setup of `tests/compiler/tests/dependency_struct_in_a_hole.nika` and runs `run` in `{tmp}/app`. (issue-1417)
* A test for "rustc warns no more" needs an empty `NAME.stderr` beside `NAME.stdout`, since stdout alone passes on the unfixed compiler too. (issue-1420)
* Never pipe the cycle's two `lower-std` runs into one grep without a marker; you cannot tell which binary raised the error, so echo a label between them. (issue-1425)
* Before planning, also probe the inherent-impl and concrete-type variants of an issue's trait or generic probe: a trait method's by-value data parameter is seeded as kept in `keeps.nika` `trait_keeps` (pending #1012 step 3), so NK2105 there is not your defect. (issue-1448)
* An impl method's contract is built by `declared_function` (`declared.nika`) from `ledger_items.nika`; a generic impl is written `impl Box[T]`, never `impl[T] Box[T]`. (issue-1448)
* Views-3 D12 (returned parameter is a view) is built only for text; a struct or generic `T` result still keeps its argument (`fn second<T>(x: &T, y: T) -> T`). (issue-1448)
* A report written through `cat > R <<EOF` runs every backticked snippet in it; quote the delimiter (`<<'EOF'`) and expand the paths separately. (issue-1458)
* Limit `git log --grep=N` with `-400` or a date range: the unbounded form can pass the 120 s tool timeout in this repository. (issue-1458)
* When an issue's probe already passes on develop, find the fixing commit with `git log -400 --oneline | grep` on the topic, pin the probe in a test program, and report fails-before as not done. (issue-1458)
* A match arm can't be an assignment (`null => x = y` fails to parse); write `null => { x = y }`. (issue-1478)
* `collections::BTreeMap` has no `extend` (NK1171), and from a function reached by a `sync` caller the refusal first shows as NK2202 "can pause"; mark the callee `sync` to see the real error. (issue-1478)
* A new `VcAsk` field also needs the Rust adapters `view_changes.rs` `analyse`, `check/host.rs` `view_change_findings` and `contracts/keep.rs` `buffer_plans`, plus `emit_contracts.nika` and the `check_walked.nika` trait. (issue-1478)
* `views_what_it_is_called_on` (`tiers.nika`) is the existing list of text methods whose result is a view of their receiver; reuse it, don't write a new list. (issue-1478)
* A list whose items come from `split()`/`map()` is a list of views of the argument, so a probe of "owned items" must build them with `a + "…"` or `push_str`. (issue-1481)
* Files of one package share one namespace: call a sibling file's `pub fn` by its bare name, since `mod x` is a parse error. (issue-1481)
* A `??` whose left side reads a map value (`m[k] ?? []`) is NK1185 in tools code; write `m[k]?.clone() ?? []` behind a `contains_key` check. (issue-1485)
* A heredoc report or edit script with backticks needs a quoted delimiter (`<<'EOF'`); unquoted, each backticked word runs as a shell command and silently drops out of the text. (issue-1485)
* A new `.stderr` expectation is written only when the file is absent before `--bless-stderr`. (issue-1490)
* A `-> ref(a) String` or `a: borrowed` in the ledger for a `String` parameter comes from the text-tier pass (`lendable_parameter`, `a_keeping_target` in `text_tiers.nika`), which runs before the keep inference and the checker. (issue-1491)
* A `.stdout`-only test passes even when the compiler prints a warning; add an empty `.stderr` beside it to pin "no warning". (issue-1491)
* Files of the views-3 migration may carry `keep` marks the old binary already warned NK1334 on; when your fix makes `keep` mean kept, drop such marks where the file means lent. (issue-1491)

