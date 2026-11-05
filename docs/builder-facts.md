# Builder facts

Fixed facts a builder would otherwise look up itself. `scripts/agent.nika`
puts *Core* into every builder's prompt, and after it the sections the brief
names (`Facts: Probes, Grammar`; a `##` heading brings its `###` sections
along). Read another section only when the work meets its topic. A new trap
goes into the section of its topic, in one line, with the run that paid for it.

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

### Every run

* **Copy `target/fast/nikaia` to your scratchpad before your first build**: it
  is the before binary for the fails-before run, and the binary long test runs
  use, since another agent may rebuild `target/fast/nikaia` mid-run (P5a, j5).
  Keep the copy in its own directory and name it exactly `nikaia`
  (`$S/old/nikaia`): script tests check the basename and otherwise fall back
  to `target/fast/nikaia` (b42). `target/release/nikaia` is older than the
  views syntax and cannot serve.
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

## Traps already met

### Tests and the gate

* **One program alone**: `nikaia test --project tests/compiler --filter NAME`
  (several `--filter`s allowed, #1238). Without `--filter` it runs everything:
  write the output to a file and grep it, never `| tail` (b933).
* **`--bless` creates a missing `.stdout`**, never a `.stderr`; `--bless-stderr`
  does both. With `--filter` it blesses only that test. A new test is run
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
* **The argument check is `check_arguments.nika`**; several early `continue`s
  there (about line 353, a view handed to a lent `String`) skip
  `the_compiler_writes_the_reference` (v18b).
* **A change that flips a tool function's signature breaks its Rust callers**
  in `crates/nikaia/src` and `tests/it`: `grep` for them before the bootstrap
  (v18b).

### Lowering and lower-std

* **Check lowered tools code with `cargo check --profile fast -p nikaia-std
  --features compiler --message-format short`** right after the first
  `lower-std`; without the feature the check is a false green (k3a2).
* **After any `*.nika` or `described.contracts` change**, run `lower-std` with
  a binary built from that tree and commit the diff; it reads and writes the
  checkout the binary was built in (set `NIKAIA_SYSROOT=/home/user/Nikaia/crates`
  for a binary built in a private copy). It lists every refusal of a file: fix
  them all before the next cycle. A new `.nika` needs one `lower-std` run
  before `cargo build`, and a row in `scripts/self_hosting.py`.
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
  fn main() {
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

* **`nikaia check --bounds FILE`** checks a file alone; a tools file is
  measured with `lower-std --bounds --asserts [--optimization
  prove-through-loops:on] > R` and `nikaia run scripts/proved.nika -- R`,
  before and after on the same tree. Where the walk stops:
  `docs/formal-verification.md` §2.
* **A test's `.bounds` keys sites by line**: an edited comment keeps its line
  count, or bless again. A precondition on a parameter's length is carried to
  the function's start (ADR-269 D20); a call is carried back only from the top
  of a body. After a precondition change run the `grep -rln 'assert('
  crates/nikaia/tests/it/` modules, not the whole `it` binary.
* Nikaia has no `i8`; widening is `i64::from`. Valgrind needs `x86-64-v3`.

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

### Syntax gaps of tools/*.nika

Classes (CLAUDE.md, *A missing form stops the work*): **[design]** holds,
build that way; **[gap #N]** stops the slice (`Result: blocked`); **[defect
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
