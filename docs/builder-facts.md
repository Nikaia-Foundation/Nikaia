# Builder facts

Fixed facts a builder otherwise looks up itself (counted over 48 builder runs,
2026-10-25: median 2, up to 19 `grep -r` calls per run to find where a name is
defined; four `pre-push-check` failures on the same badge; seven runs puzzled by
the same dirty lock file). Read once; the brief adds only what is specific.

## Where things live

| What | Where |
| :--- | :--- |
| Checker, moved parts | `crates/nikaia-std/src/tools/check_*.nika` (87 files); Rust adapter in `crates/nikaia/src/check/` |
| Emitter | `crates/nikaia/src/emit/mod.rs`, `emit/host.rs`; moved parts `tools/emit_*.nika` |
| `std` methods by receiver | `std_receivers` in `tools/check_core_method.nika` |
| Generated Rust of a tools file | `tools/lowered/<file>.rs`, index `tools/lowered.rs` (never edit, `lower-std` writes them) |
| Program tests (new tests) | `tests/compiler/tests/NAME.nika` + `.stdout` / `.stderr` |
| Rust tests by NK code | `grep -rln NK1234 crates/nikaia/tests/it/` |
| Specification | `docs/specification/10-nikaia-light.md`, `20-nikaia-advance.md`, `30-nikaia-tooling.md` (code table) |
| A name, Rust or Nikaia | `nikaia run scripts/where.nika -- NAME...`: definitions (`fn`, `struct`, `enum`, `type`) with file:line, the files that call it; for an `NKnnnn` the tests and the spec row (a field is not found: grep it) |

## Traps already met

* **A wrong lowering: look before you read code** (b933, 15 turns reading `views.nika` before one probe found the cause): `nikaia check --tethers F.nika` prints what each result views, and the `nikaia.contracts` file written beside a lowered probe shows the ledger the lowering used. Run both on the probe first; they name the pass (`buffers.nika`, `tether.nika`, `views.nika`).
* `nikaia test` has no name filter: write its whole output to a file (`> out.txt 2>&1`) and grep that, never `| tail` (b933 ran `tests/compiler` a second time, 10 minutes, to find one test's line).
* `tests/compiler/nikaia.lock` is git-ignored, like the other test
  packages' locks (it changes with every compiler build). Never commit it.
* `scripts/pre-push-check.sh` fails with *out of date: self-hosted.svg* when a
  `.nika` changed the self-hosting share: run `python3 scripts/badges.py` and
  add `assets/badges/self-hosted.svg` to `Files:` (the only badge you touch).
* `cargo test ... --test it -- a:: b::`: the `--` before the filters is needed.
* `export A=1 && S=x cmd &` sends the whole chain to the background, and the
  output is lost (P4b ran clippy and its tests twice): put the `&` on `cmd` alone.
* A `nikaia run` script cannot `use` another script and cannot find its own
  binary; a test program cannot set `RUSTFLAGS` or see git state. Such a
  change is checked by hand runs; say so under `Deviation`.
* Valgrind cannot run a binary built for an AVX-512 CPU: name `x86-64-v3`.
* Builders have no GitHub: the brief holds the issue text. If it does not,
  say so under `Deviation` and build from the brief's title only.
* Syntax gaps of `tools/*.nika`: the list below, *Moving Rust into Nikaia*;
  `crates/rs2nika/convert --lint F.nika` flags the known ones.
* Guard tests (`range_spelling`, `shipped_ledger`, the std ledger in
  `contracts`) scan the whole repository: a script's `a..b` or a new
  `unsafe fn` fails them. They are in the gate (builder.md) for that reason.
* **CI has no `target/fast`.** CI builds `target/release` only; a test that runs the toolchain takes the `nikaia` that runs it (`readlink /proc/$(cut -d' ' -f4 /proc/$PPID/stat)/exe`), never a fixed `target/fast/nikaia` (four script tests kept develop red that way, 2026-10-26). A red test that passes locally: check what CI lacks first.
* **A parser change runs the whole `it` binary** (`cargo test --profile fast -p nikaia --test it`, no filter): many `it` tests hold one-line programs as Rust strings, and `accepted_compiles` lowers the corpus; 9936c70 ran `-- parse` only and turned `develop` red (2026-10-26).
* **A change to `scripts/X.nika` runs the tests that name it** (`grep -l X tests/compiler/tests`), also from the main session: a `--dry-run` line added without its `.stdout` kept `develop` red (7970db8, `drive_actions`, 2026-10-26).
* **A new `tests/compiler` test is run through `nikaia test --project tests/compiler`, never only with `nikaia run`** (7c1643d, 2026-10-29: `print(m)` writes `1` without a newline, the `.stdout` held `1\n`, CI went red on bootstrap_compiler and arm64). `print` writes no newline, `println` one; the `.stdout` holds the bytes exactly.
* **Test with your own copy of the binary.** In the shared checkout another agent may rebuild `target/fast/nikaia` mid-run: after your build, `cp target/fast/nikaia "$SCRATCH/nikaia"` and run long tests (`nikaia test --project tests/compiler`) with that copy (P5a lost two runs, 2026-10-28). An untracked test another agent is writing may fail: name it under `Tests:`, do not touch it.
* **`[forms X_]` rows in `rs2nika.names` match the Nikaia draft text, not the Rust** (P5b, 2026-10-29: three re-converts to learn it; P5d's rows written against the Rust matched nothing): copy the left side from the draft as it came out (`eprint(`, `host.env_var(M) != null || ...`); `{x}` stands for a dotted receiver chain only (`a.b.c`), not a call result. Before step 1 relies on rows, add one row, re-convert, and check the draft changed (one probe), then write the rest.
* **`.map(fn(x) { x.clone() })` in `tools/*.nika` lowers as written and trips `clippy::map_clone`**: the tools module in `crates/nikaia-std/src/lib.rs` allows it since P5d (0f9a525); another lowered lint goes into the same allow list, never into the lowered file.
* **A converter draft of `impl` methods, when the brief wants free functions**: keep the bodies, redo the signatures at once (`self.x` -> a parameter); do not edit method by method (P5a kept 6 of 104 lines, 2026-10-28).

## Moving Rust into Nikaia (self-hosting, ADR-294, #125)

Rules from the sessions that moved the most; they save tokens, not just time.

* **Pick the slice by the call graph, leaves first.** The `Checker` and the
  emitter's state become Nikaia structs (#558, owner's direction: C directly,
  no described Rust type in between). Rust's struct holds the Nikaia state as
  one field and a method moves into an `impl` in Nikaia once everything it
  calls has moved; the large recursive core (`value_of`, `stmt`, `call`;
  `expr_as_written`, `a_stmt`) moves in one step. `grep 'Finding {'` and
  `format!(` find the cheap message-building first. A walk that cannot move yet
  takes a callback (`fn(ref Expr) -> i64 sync`; see `bounds_basic.nika`). Code
  keyed on expression addresses waits for the expression id (#558); changing a
  `mut` parameter in a `match` is ADR-094 D7 / #576.
* **Copy a pattern, do not invent one.** `libraries.nika` + `libraries.rs`
  (small), `assets.nika`, `parse_numbers.nika` (callbacks), `interpreter.nika`.
  The Rust module stays as the adapter that calls `nikaia_std::tools::<m>::f`.
* **One namespace for all `tools/*.nika`.** Type and function names are global
  (`Decision` clashed): prefix them with the module. Calls to other tools files
  are unqualified.
* **A new `.nika`** needs one `lower-std` run with the old binary before
  `cargo build` (the module only appears in `lowered.rs` after that run), and a
  row in the table in `scripts/self_hosting.py`. The share counts code lines,
  not comments.
* **After any `*.nika` or `described.contracts` change**, run `lower-std` with a
  binary built from that tree, or the sysroot test fails on a stale
  `lowered/*.rs`. `lower-std` reads and writes the checkout the binary was
  **built in** (`CARGO_MANIFEST_DIR`, unless `NIKAIA_SYSROOT` says otherwise),
  not the working directory. A lowering change can change the generated files:
  run `lower-std` after building and commit the diff. It reports every refusal
  of a file, in file order, as `nikaia check` does: fix them all before the next
  cycle.
* **Reproduce a lowering defect in seconds**, not with a cargo build:
  `target/fast/nikaia lower f.nika` or `nikaia run f.nika`. A defect that shows
  only in `tools/*.nika` needs a throwaway probe function in a tools file and
  `nikaia lower-std`; look at `tools/lowered/<file>.rs`, then restore both.
* **Syntax gaps** (each cost a build cycle):
  * no `if let`: use `x ?? default`, or `starts_with` then `strip_prefix(..) ?? ""`;
  * no `*x`; loop variables over a `ref Vec` are values;
  * `.to_owned()` is `NK1189`: write `.clone()`;
  * text is `String` (ADR-282 D2); never `ref` on a number, `bool` or a
    data-less enum (`NK1257`). Until #654 step 0 is built, `ref String` is
    written in exactly these places and nowhere else: a field; a type argument
    (`Vec[ref String]`, a map's key); an option after `;`; a function's or a
    rule's result; a parameter the result points into; a parameter used as a
    key of a map keyed by `ref String`. Everywhere else, above all a
    parameter that is only read, it is `String`;
  * `opt ?? ""` moves the option: `(opt ?? "").clone()` for a second use;
    a method on a `String?` is `opt?.len() ?? 0`;
  * f-string braces are `{{` `}}`; a plain string takes braces literally;
  * a function parameter is `fn(ref X) -> T sync`; a `Span` that arrives by
    reference is copied as `Span { start: s.start, end: s.end }`.
  * `view_of` resolves to `signature::view_of`, not `check_types::view_of_type`:
    a false `NK1104` (*returns `ref String`, but declared `ref String`*) means
    the wrong one was called;
  * a nested `Some(FactWrap::Widen(x))` pattern leaves `x` untyped and counts
    as an unanswered method call (`sequences::`, ceiling 17): bind
    `how ?? ...` first and match the plain enum;
  * a `T?` local needs `?.clone()` (NK1125);
  * a `ref String?` parameter does not accept a bare `String` argument: pass a `String?` local;
  * `for mut x in v` is not accepted;
  * `a..b` includes `b` (Part I, ranges); the half-open range is `a..<b`;
  * the catch-all arm of a `match` is `else =>`; `_ =>` and `None =>` are refused;
  * a `String?` field needs `x?.clone()` before it can be matched.
  * collections (P4b): a `BTreeMap` is walked with `for (k, v) in m` (no
    `.iter()`), a `BTreeSet` has no `extend` (`insert` in a loop); `fs::read`
    is async and gives `Bytes` (the Rust caller needs `block_on`; a `Vec[u8]`
    is filled by a loop, as `sysroot_cache.nika` does); `read_dir` gives sorted
    names, not paths. Two host parameters `[M: A, H: B]` lower.
  * a text literal changed in place needs its type: `let mut out: String = "a"`
    (without it `lower-std` reads `String?`, NK1125/NK1104, #943).
* **Host traits** (M3b, 2026-10-29): a host method that `throws` lowers to
  `Result<T, Box<dyn std::error::Error>>`; to hand an `anyhow` refusal back
  unchanged, keep it in a `RefCell` of the Rust host and return it after the
  call. A body-less trait method's text parameter reads as owned: write
  `ref String`. Rust state the host keeps (checked units, a source map) is a
  `RefCell` field; the Nikaia side names units by `i64` index.
* **Reading an `it` run:** filter with `grep -E '^test result:|FAILED|panicked'`,
  anchored: a bare `test result` also matches test names such as
  `result_source::` and hides the summary (M3b repeated a 2-minute run).
* **Lowered types:** `String` is `&str` where a view suffices and `String`
  where it is owned (the compiler decides, ADR-282 D2), `ref T` is `&T`, `Vec[String]` is
  `Vec<String>` (a `ref` one a slice), `String?` is `Option<String>`. Most Rust
  errors after a move are `&` against owned. Check that `Display` equals
  `Ty::text()` before replacing `{ty}` with it.
* **Moving a resolver or a map loop:** never compare a contract by address
  (`std::ptr::eq`): a resolver that returns an owned value breaks it silently.
  Destructure map loops (`for (name, value) in map`), not `entry.0`/`entry.1`,
  which the checker cannot type; a named `_` (`for (name, _) in map`) avoids the
  unused-variable clippy error in the lowering. A layout change breaks tests
  keyed on an address by allocator luck.
* **Delete the dead locals** a replaced block leaves behind (`cargo check` lists
  them as unused), and re-grep function names instead of trusting line numbers
  from an earlier listing: every commit shifts them.
* **Edit scripts:** replace blocks bottom-up with exact-string asserts. Never
  `str.replace('', ...)` (an empty-slice replace corrupted `emit/mod.rs` once).
* **The converter** `crates/rs2nika/convert` (a Nikaia project: `nikaia build`
  there, then `target/nikaia/bin/rs2nika [--profile check|check-free|emit] [--prefix X_]
  [--file] F.rs`; `check-free` for code without a `Checker`, `Emitter` or `self`:
  no `sync` on free functions, R85) writes a first draft of a Rust function in Nikaia (TODO
  markers where it cannot), names on stderr the callees not yet in Nikaia and
  the functions tools/*.nika may already have; `--rules` lists its rules,
  `--lint F.nika` flags known defects in hand-written Nikaia, and
  `rs2nika.names` beside it maps Rust helpers to their Nikaia names;
  `--score DRAFT FINAL` prints how much of a draft the committed code kept
  (the report's `Rs2nika:` field). `--out DIR A.rs B.rs ..` converts many
  files in one run, on all cores, reading the repository once: `DIR/<name>.nika`
  and `DIR/<name>.notes` (the stderr lines), each prefixed `<name>_` unless
  `--prefix` is given; `--corpus` also scores its cases in parallel. It writes the draft from
  rust-analyzer's syntax tree (`--engine tree`, the default; `--out` and
  `--file` still take the line engine, `--engine lines`). Iterator chains (`.iter().map(..).sum()`,
  `.any(..)`, `.filter(..).map(..).collect()`) are the house style in
  `tools/*.nika`, as the converter writes them (owner, 2026-10-27). Rust's
  types come from rust-analyzer (ADR-001 D5, `crates/rs2nika/facts`): `--facts
  F` takes them (`rust-analyzer lsif crates/nikaia > x.lsif`, two minutes,
  then `nikaia run -- x.lsif > F` in `crates/rs2nika/facts`; delete the 100 MB
  `x.lsif` after), and closures get their types and options their `match`
  from them (R82, R222). **What a move decides goes into `rs2nika.names`**
  (R224): a name the Nikaia code spells otherwise (`old new`), and under
  `[types]` a Rust type it writes otherwise (`Rc<RefCell<Node>> = NodeId`,
  matched without spaces), each with its reason after ` #`, in the commit of
  the move, so the next draft already carries the decision. A project of its
  own keeps its table in the directory it converts from (or `--names FILE`);
  it is read before the converter's; open
  lowering defects are GitHub issues, the language options against the usual
  hurdles in `docs/history/mitigation-hurdles.md`.

## Commands, exactly

The cycle, clippy beside the tests, the report and `commit.nika`:
`.claude/agents/builder.md`. Disk at the end: the same file, last step.
