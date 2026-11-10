# Builder facts: lookup

The traps already met and the rules for moving Rust into Nikaia; the short
*Core* every builder gets is in `docs/builder-facts.md`. Nobody reads this file
whole: `nikaia run scripts/syntax.nika -- KEYWORD` prints the lines of a topic
(with the spec section and an example test), and `scripts/agent.nika` puts the
sections a brief names (`Facts: Probes, Grammar`) into the prompt. A new trap
goes into the section of its topic, in one line, with the run that paid for it.

## Traps already met

### Parser nodes that change

* **`Expr::AsyncGroup` gains two `Option` fields** with #986 (the async
  lane): a new match on it in `tools/*.nika` is written with
  `..`, so it does not break when the fields land.

### Tests and the gate

* **One program alone**: `nikaia test --project tests/compiler --filter NAME`
  (several `--filter`s allowed, #1238). Without `--filter` it runs everything:
  write the output to a file and grep it, never `| tail` (b933).
* **`--bless` creates a missing `.stdout`**, never a `.stderr`; `--bless-stderr`
  does both. Exception: a program the compiler refuses (a refusal test) is
  blessed by plain `--bless` into its `.stderr` only, no `.stdout`; so a test
  meant to run that `--bless` turns into a `.stderr` is a refusal to fix, not
  a missing flag (`project_tests_bless_program`, #1238; issue-1431). With `--filter` it blesses only that test. A new test is run
  through `nikaia test`, not only `nikaia run`: `print` writes no newline and
  the `.stdout` holds the bytes exactly (7c1643d).
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

### Probes

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

### Checker and views

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
* A check run over `value_leaves` after the arms are checked must not re-type a leaf with `check_core_expr` unless its root is a parameter (`a_place_of_a_lent_parameter`), or names a `match` pattern bound give NK1117 in `ast.nika` during `lower-std`. (issue-1425)
* A `pub fn` with a mixed `String` result keeps it owned (ADR-282 D22), so a borrowed-field arm needs the per-arm NK1131; probe both the tail form and the `return if` form. (issue-1425)

### Lowering and lower-std

* `commit.nika` stages whole files: a defect fix that shares a file with a step is split by putting HEAD's version plus the fix hunks (in the `.nika` and its lowered `.rs`) in place, committing, then restoring the step. Plan the fix as its own commit before starting the step in that file. (issue-1470)

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
* `lower_tools_files_cached` re-parses files with `test` blocks (`with_tests_as_functions`), so package-wide values computed from `parsed` do not cover those files' test functions. (issue-1279)
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

### Grammar

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

### Scripts

* **Forms** (p1): text to integer `text::parse_i64(s)`; arguments `for a in
  cli::args() { args.push(a) }`; a text is cut only at a position `find`
  returned (NK1214), else `strip_suffix(..) ?? ""`.
* **A change to `scripts/X.nika` runs the tests that name it**: `grep -l X
  tests/compiler/tests` (7970db8).
* **A script cannot `use` another script or find its own binary**; a test
  cannot set `RUSTFLAGS` or see git state: check by hand runs, say so under
  `Deviation`.

### Prover

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
* **Before adding debug prints to `bounds_*.nika`, run `nikaia check --bounds --bounds-trace LINE F.nika`** (`lower-std --bounds --bounds-trace FILE:LINE` for a tools file): for each goal of a statement starting on that line it prints the goal, every fact it was asked with (rule facts and ledger facts included), how many the 48-fact cap evicted, and the type's field rules. (issue-1495)
* **A rule's fact over every element of a list is guarded by the list's emptiness**: `bounding_each_guarded` turns it into `len(xs) <= 0 || fact`, and `bounding_each_present` gives `len(xs) >= 1` where an element is actually read. A new kind of rule (as D28's `inner`) states its element facts through these, or it proves anything under an empty list. (issue-1483)
* **A list field that a free function such as `main` writes is dropped from the rule candidates**, so a rule test fills its lists only through `ref mut self` methods. (issue-1483)
* **Derived entry claims (ADR-391) key on the first non-`self` parameter**, so a method that takes only `self` gets none. (issue-1483)
* **`nikaia check --bounds FILE`** checks a file alone; a tools file is
  measured with `lower-std --bounds --asserts > R` and `nikaia run scripts/proved.nika -- R`,
  before and after on the same tree. Where the walk stops:
  `docs/formal-verification.md` §2.
  The two can differ: `check --bounds FILE` proved a site D23 reaches that
  `lower-std --bounds` did not (#1465); a figure is claimed only from
  `lower-std`. (issue-1414)
* **A `for` binding is a number of the walk** (`CheckFacts.whole_loops`):
  check it before blaming the prover when a guard on `for k in ks` is lost.
  (issue-1414)
* **A test's `.bounds` keys sites by line**: an edited comment keeps its line
  count, or bless again. A precondition on a parameter's length is carried to
  the function's start (ADR-269 D20); a call is carried back only from the top
  of a body. After a precondition change run the `grep -rln 'assert('
  crates/nikaia/tests/it/` modules, not the whole `it` binary.
* Nikaia has no `i8`; widening is `i64::from`. Valgrind needs `x86-64-v3`.
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

## Moving Rust into Nikaia (self-hosting, ADR-294, #125)

### Slices and patterns

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
  the adapter that calls `nikaia_std::tools::<m>::f`.
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

* `grammar` is a reserved word: a binding named `grammar` in `tools/*.nika` is refused by `lower-std`. (issue-1431)

Classes (CLAUDE.md, *A missing form stops the work*): **[design]** holds,
build that way; **[gap #N]** stops the slice (`Result: blocked`); **[defect
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
#N]**: fix it or wait; **[works]**: use it, do not copy an old way around.

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
* [gap: ADR-views-2 §6] `mut` on one name of a tuple (`for (k, mut v) in m`),
  and so `values_mut()` / `iter_mut()`, is `NK1307`: stops the slice;
* [design: Part I 3.3, 3.4] `a..b` includes `b`, half-open is `a..<b`; the
  catch-all arm is `else =>`;
* collections: [works] `m.iter()`, `for (k, v) in m`, `a.iter().zip(b.iter())`;
  in-place change of a map value `let mut c = m[k] ?? continue` (#952); [gap
  #950] no `BTreeSet::extend`, no `BTreeMap::entry`; [gap #949] no
  `mem::take`; [design: Part I 2.6] a text is cut only where it handed out a
  position (`NK1214`): `hex.scalars.take(16).collect()`; [design: Part III]
  `fs::read` is async and gives `Bytes`; `read_dir` gives sorted names.

### Lowered types and host traits

* `String` is `&str` where a view suffices and `String` where owned (ADR-282
  D2), `ref T` is `&T`, `String?` is `Option<String>`. Most Rust errors after a
  move are `&` against owned. Check that `Display` equals `Ty::text()` before
  replacing `{ty}`.
* A host method that `throws` lowers to `Result<T, Box<dyn std::error::Error>>`;
  keep an `anyhow` refusal in a `RefCell` of the host to hand it back. A
  body-less trait method's text parameter is written `ref String`. Host state
  is a `RefCell` field; the Nikaia side names units by `i64` index (M3b).
