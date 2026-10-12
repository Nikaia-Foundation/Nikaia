## Supplement (2026-10-11): from a draft script to `nikaia import rust`

**Where this comes from.** The owner's direction today: the converter of this issue becomes a user-facing command, `nikaia import rust`. It uses the same machinery as the draft script, brought to the highest fidelity that can be proven, and it works on programs outside this repository. The self-hosting move stays its first customer and its test bed. This replaces the section *Not decided here*. Nothing below changes the language. Section 4 lists the owner's decisions, and each one needs its own record. Markers: **[V]** checked today on `origin/develop` 72363084 with its binary (a probe or the code); **[M]** from memory, not checked; **[U]** unknown.

### 0. The feature, from zero

Today `scripts/rs2nika.py` reads Rust a line at a time, applies 52 rules and prints a draft, which a builder finishes by hand. **[V]** Example:

```rust
fn sum_to(n: usize) -> usize { let mut s = 0; for i in 0..n { s += i; } s }   // prints 45 for 10
```
The draft is `fn sum_to(n: i64) -> i64 sync { … for i in 0..n { s += i } return s }`. It passes the checker and prints **55**. In Nikaia, `0..n` includes `n` and the exclusive range is written `0..<n` (ADR-291 D13). rs2nika has no rule for ranges. A builder who reviews every line catches this. A product would hand the user a wrong value without a word. That is the design question in one line: **in the output, every line either means what the Rust meant, or it is refused with a reason, or it carries a named obligation that a check discharges.** On the one piece of human-written Rust in the repository that has a Nikaia twin with the same output (`benches/brc/src/bin/naive.rs`, twin `examples/1brc.nika`), the draft **does not parse**, and none of its 3 functions is usable as written **[V]**: `if let` inside `let (..) =`, `std::env`, `entry()`, `impl Iterator`, tuple fields. And `format!("{}={:.1}", name, min)` became `f"{name}={:.1}"`, which **dropped the arguments without a word**.

The command: `nikaia import rust <crate|file>` reads a crate and writes Nikaia. It checks the Nikaia, lowers it back to Rust, compiles that with rustc and runs the crate's own tests against the result. Then it prints, per function, what was proven, what was tested and what was refused.

### 1. What "perfect" can mean

| Level | The claim for one function | Checked by |
| :--- | :--- | :--- |
| L0 | the front end read it | the parser |
| L1 | Nikaia accepts it | `nikaia lower` (there is no `nikaia check` subcommand in 72363084 **[V]**) |
| L2 | the lowered Rust compiles | rustc. L1 without L2 is a compiler defect (Part III C.1), never the user's |
| L3 | same behaviour on tests | the original binary and the round-trip binary on the crate's tests and recorded inputs: stdout and exit code byte-identical |
| L4 | proven | the round-tripped Rust equals the original after normalisation, modulo a fixed list of rewrites, each proven once and tested (translation validation **[M]**: Pnueli et al. 1998, Necula 2000) |

* **"Perfect" is defined as:** no function claims a level it did not reach, and **no function reaches L2 and fails L3** (no silent change). The pass rate comes second. Whether two programs are equivalent is undecidable, so L4 is a per-function result on the subset where normalised equality holds, never a claim about a whole program. **[V]** The hand-fixed `sum_to` lowers to `pub fn sum_to(n: i64) -> i64 { let mut s: i64 = 0; for i in 0..n { s += i; } s }`. That is the original except for `usize`→`i64`, and that difference is exactly an obligation (below).
* **The baseline is Rust with overflow checks on.** **[V]** `i32` 2147483647 + 1 wraps under `rustc -O` and aborts in Nikaia (Part I 2.2). `vec![].len() - 1` gives 18446744073709551615 in a release build, a panic in a debug build, and `-1` in Nikaia (a length is `i64`). So L3 runs the original with `-C overflow-checks=on`, and a test that passes only by wrapping is reported.
* **Corpora** **[V]**:
  * the gold pairs: 53 commits matching this issue's grep (`E[0-9]|C-[0-9]|#558`) touched `tools/`. Each is Rust before and Nikaia after, reviewed and passed by CI. The oracle is the suite plus `scripts/c6/oracle.py` (142 files, byte-identical).
  * `benches/**`: 3,489 lines of human-written Rust, with `naive.rs`/`1brc.nika` as the one pair whose outputs are equal.
  * `tests/**` and `examples/**` (142 `.nika` files), lowered to Rust: Rust with a known Nikaia source. That Rust is machine-written (`nikaia_std::index::or(..)`), not human style.
  * **The bias is measured:** the compiler's remaining Rust has **1** `for … in a..b` (the tools have 226 `..<`), 72 `as usize`, 21 `unsafe`, 27 `dyn`, 22 `Cell`/`RefCell` and 3 `macro_rules!`. The self-hosting corpus under-represents what outside programs write. An outside corpus is chosen, and its prediction committed (ADR-294 D15), before Phase 3 measures.

**Rust constructs.** E = exact. P = mapped with a proof obligation that the round trip must discharge. R = refused, with the reason named. D = needs a language decision, and is refused until it is taken. The importer never decides what the language offers (CLAUDE.md: decided from what Nikaia is for, not from what the importer meets).

| Rust | Nikaia | | Fact |
| :--- | :--- | :-: | :--- |
| `bool i32 i64 u8 u32 u64 f32 f64`, `as` casts | the same | E | Part I 2.2. **[V]** `300 as u8` = 44, `-7/2 -7%2` = `-3 -1`, float printing equal. rs2nika R10 widens `i32`→`i64` (a self-hosting convention, wrong for the product) |
| `usize isize` | `i64` | P | wrap/underflow differ **[V]** (above). The pub API changes |
| `i8 i16 u16` | `i32`/`u32` | P | the overflow point moves |
| `i128 u128` | — | R | `NK1135`. ADR-294 D9.2 declined `i128` |
| `char`, `b'x'` | `scalar`, `'x'` as `u8` | E | Part I 2.6 |
| `&str`/`String`, `s.len()` | `ref String`/`String`, `s.len()` | E | **[V]** `"äb".len()` = 3 bytes in both (Part I 2.6 says *there is no `s.len()`*, and the checker accepts it: report it as a spec/compiler mismatch) |
| `&s[a..b]` on text | `s[a..<b]` | P | `NK1214`: only positions the text handed out cut it |
| `a..b`, `a..=b` | `a..<b`, `a..b` | E | ADR-291 D13. Missing in rs2nika: the silent bug above |
| `Option<T>`, `None`, `Some(x)` | `T?`, `null`, `x` | E | ADR-278. `(A, B)?` → a named struct (P, lowering defect D5) |
| `Result<T,E>` returned, `?` | `-> T throws`, an error enum + `impl Error` | E | Part I 7.1 |
| `Result` stored as a value | — | P | a program never holds a `Result` (ADR-290 D21): rewritten as an enum |
| `Vec`, `BTreeMap/Set` | `Vec[T]`, `collections::BTree*` | E | |
| `HashMap/Set` | `collections::HashMap` | P | decision D: the hasher is the keys' trust's, so iteration order may differ; a custom `S` is dropped; a `pub` signature gains a hidden hasher parameter (default `Keyed`, std's own) |
| `Box<T>` | `T` | E | the compiler boxes (ADR-246) |
| `Rc<T>`/`Arc<T>` | `Shared[T]` | P | Part III 15.2. `Weak`, `ptr_eq`, `get_mut` **[U]** |
| `Rc<RefCell<T>>`, `Arc<Mutex<T>>` | `SharedMut[T]` | P | the four doors. A `Seen` stamp refuses blind writes, and a lock inside a lock or I/O under a lock is refused (Part I 6.3, Part II 12.2-12.3) |
| `Cell`, `RefCell` alone, `UnsafeCell`, atomics | — | R | interior mutability outside a lock. Atomics **[U]** |
| `&T` / `&mut T` parameter | `ref T` / `mut x: T` | E | **[V]** `mut v: Vec[i64]` lowers to `&mut Vec<i64>`, and the caller sees the push |
| `struct S<'a> { s: &'a str }` | `struct S { s: ref String }` | E | **[V]** runs (ADR-283 stored views) |
| result tied by `'a` to one of ≥2 refs | — | D | **[V]** `NK2302` even when every `return` hands back `a`; accepted once ADR-283 D25 is built (decision B) |
| `-> &mut T` | — | R | **[V]** `NK1158`, *`ref mut` only when talking to C* |
| tuple struct `S(i64)` | named fields | P | **[V]** does not parse |
| generics, bounds on own traits | `[T]`, `[T: Tr]` | E | Part I 4.6-4.7 |
| bounds on std traits (`Ord Hash Clone Display Default Iterator`…) | — | D | **[V]** 10 of 10 refused, `NK1135` *no trait called …* |
| `derive(Clone, Copy, PartialEq)` | implicit | E | ADR-294 D9.1 |
| `derive(PartialOrd, Ord)`, `sort` of structs | — | D | ADR-294 §6, ADR-204 §4. **[V]** `sort_by` `NK1171` |
| trait default body | copied into each `impl` | P | **[V]** no default body (Part I 4.7) |
| associated types, operator overloading, `dyn Trait` | — | R/D | **[V]** parse error / `NK1191` / parse error |
| `Box<dyn Fn(A)->B>`, closures | `fn(A) -> B` | P | **[V]** returned closure runs, mut capture runs. Capture rules Part I 5.4 |
| `impl Trait` (argument / return) | `[T: Tr]` / — | P / R | **[V]** return position does not parse |
| inherent `impl` on a foreign type | trait + `impl` | P | **[V]** `NK1209` says so |
| `impl Drop` | `impl Cleanup` | P | `NK2601` |
| labelled `break`, `loop` with a value, `while let` | flag/helper `fn`, `match` | P | **[V]** none parse. `NK1133` |
| `if let`, `let else`, `matches!`, `_ =>`, shadowing | `match`, `?? return`, `else =>` | E | rs2nika R14/R22-R24. **[V]** shadowing runs |
| `println! format! write! vec! assert!` | f-strings, `[..]` | P | format specs (`{:.1}`, `{:?}`, `{:>8}`) per spec, never dropped. **[V]** `println!(..)` passes the checker and fails in rustc: a C.1 defect |
| `macro_rules!`, proc macros, `derive(Serialize)` | — | R | no macros (Part II). **[V]** expansion needs `-Z` (refused on stable) |
| `unsafe`, raw pointers | — | R | `unsafe` only around an `extern` call (Part I 2.1) |
| `extern "C"` | Part III 15.1 | P | |
| `thread::spawn`, `async`/`.await`, `join!`/`select!` | `spawn`, pauses by default, `overlap`/`select` | P | runtime APIs outside a table → R |
| std calls | by a table against `std.contracts` | P | 409 entries, 24 on `Vec` **[V]**. Unmapped → R, or stays Rust |

The 19 entries of `docs/lowering-defects.md`: **none changes behaviour without a word.** 12 are rustc errors after the checker accepted (D1-D3, D8, D10, D11, D11b, D14, D15, D17-D19), which L2 catches. D7 is clippy only. D4 and D6 are refusals or unstable types. The rest are parse or by design. New from today's probes **[V]**, not yet in that file: `println!(..)` passes the checker, and `xs.iter().filter fn(x) { x % 2 == 0 }` passes the checker and fails rustc (`&&i64`). **The silent bugs are the converter's (ranges, format arguments), not the lowering's.**

### 2. Architecture

**The front end (Rust parser).** These are the options:
* **(a) Extend `tools/rust.nika`.** Today it is 624 lines. It reads signatures only, crosses bodies as balanced text, has *"not a type grammar"* (its header), and has no raw strings or nested comments. No full expression grammar has been written as a Nikaia grammar yet: Nikaia's own parser is Rust (`parser/`, 4,037 lines), and it is exponential on nested parentheses, `if`s and blocks (`selfhosting-plan.md`: 16 levels, 0.6-1.7 s). Whether the grammar backend shares that is **[U]**. This option fits ADR-290 D11/D13.
* **(b) `syn`.** It is already in `Cargo.lock` (2.0.116, as a build-time dependency of a derive). Its features are **[U]**, and it parses bodies **[M]**. ADR-290 D13 rejected it for `describe` because *the command is a Nikaia program, so its parser is a grammar*.
* **(c) rustc's HIR/MIR, rustdoc-JSON, `-Zunpretty=expanded`.** **[V]** All are refused on stable 1.97. ADR-001 D1 rules them out.
* **(d) rust-analyzer as an external tool.** It is not installed here (**[V]** the rustup proxy has no such binary). It is a stable rustup component with index dumps (SCIP/LSIF) **[M]**. It gives types, at the price of a tool the user has to install.

**Typed or untyped.** The question is which decisions need Rust's types. **[V]** Ownership needs none: views are inferred (Part I 2.2), a struct whose parts all copy copies (ADR-294 D9.1), a `mut` parameter is `&mut`, and every move that is valid in Rust stays valid. Two things do need types: **the std call table by receiver type** (`get` on a `Vec` or on a map, `min` on `i32`, iteration over a map) and **integer widths of intermediate expressions** (the `usize` probe). **Recommendation:** convert by syntax; let Nikaia's checker decide ownership in a *convert, check, repair* loop (`nikaia lower` reports every refusal with its span, and `NK1245` marks a type nobody can tell); take integer types from what is written; and repair on rustc's errors, which ADR-300 already translates. rust-analyzer only if the std table's measured miss rate demands it.

**Ownership, the central question.** A Rust lifetime is *information*: `fn pick<'a>(a: &'a str, b: &str) -> &'a str` says where the result points. Nikaia writes no lifetimes (ADR-283 D4) and infers them, and with two view parameters it refuses (`NK2302`). The importer then has three ways out:
* `.clone()`: an allocation per call, which no callgrind run has counted yet;
* a `Held` struct: more lines;
* a refusal.

Decision B settles it: one source on every way out compiles (ADR-283 D25); the count is in `history/mitigation-hurdles.md` §1. The report counts every clone the importer adds as a performance obligation.

**Partial import.** What is refused stays Rust. The Nikaia program calls it as a described path crate (`type = "rust", path = …` plus `contracts/`, **[V]** `examples/foreign-runtime/crossing`), and Rust calls the imported Nikaia as a module (ADR-290 D16). This is the strangler that self-hosting uses with its host traits. Its limit: a foreign type's methods are not described (ADR-290 §6).

### 3. Verification, built in

The steps, per crate: read → convert → `nikaia lower` (L1) → `cargo build` of the lowered package (L2) → the original tests against both binaries (L3) → a normalised comparison of the original and the round-tripped Rust per function (L4). **The oracle has two binaries**, like `scripts/c6/oracle.py`'s two compilers. The lowered program is an ordinary Rust module (ADR-290 D16), so the original and the round trip can be linked into one test binary, and a differential harness calls `f_orig(x)` and `f_rt(x)` on the same inputs. **API fidelity:** `rust.nika` reads the `pub` surface of both crates **[V: it reads `pub fn` and the type text]**, and the diff shows `usize`→`i64` as an API change, for which the original tests need an adapter. **The report is a table per function:** level reached, rules applied, obligations open and how each was discharged, clones added, refusal code. **Failure:** the product output has no TODO markers (they stay in the builder's draft). A function is imported at the level it reached, or it is not imported and stays Rust with its code. **Codes:** `NK28xx` is unused today **[V grep]**. A new Part III C.3 row, *importing from Rust*, with one code per R reason in the table above, is written by the deciding session (decision G).

### 4. What it needs from the language and the rules (owner decisions)

Each topic below goes into its own decision round. Nothing here blocks the self-hosting move. Where they come from: B-E are in `docs/history/mitigation-hurdles.md`, which the orchestrator of the #125/#558 sessions wrote; it *recommends* them and nobody has decided them. A and F-I are new from this round. Only I is needed now (it decides who builds what). A is needed before Phase 3, and F-H before the first `NK28xx` code ships.

**A. What the importer is written in and what reads Rust.**
* *What it is:* the command needs a parser for all of Rust. ADR-290 D11 says the toolchain's tools are written in Nikaia, and D13 says the parser is a grammar.
* *Origin:* this issue's step 3 ("decide from a measurement"). ADR-290 D15 says that where a principle decides, a number is only a diagnosis. Both cannot rule.
* *Facts:* (a)-(d) in §2.
* *Risk:* a grammar for full Rust is the largest single cost in this plan. `syn` reopens a choice the ADR made.
* *Recommendation:* D11/D13 apply. The front end is a grammar extending `rust.nika`, built only as far as the corpus needs. The measurement in step 3 becomes a diagnosis.

**B. NK2302 precision** - **decided** (October 11, 2026): [ADR-283](specification/adr/adr-283.md) D25-D26, Part I 6.6, issue #580.
* The result points into at most one parameter, read off the body. A Rust signature that ties the result to one of several references (`fn pick<'a>(a: &'a str, b: &str) -> &'a str`) imports without a clone once its body hands back only `a`.
* One lifetime over several references (`fn longest<'a>(a: &'a str, b: &'a str) -> &'a str`) where the body returns either is `NK2302`: the importer refuses it, or splits it, or adds a `.clone()` and counts it as an obligation.
* A trait method or a function type follows the elision (D26); one with no receiver and two references in a reference result is refused.

**C. An order on types and std traits as bounds.**
* *What it is:* `fn max<T: Ord>` and `xs.sort()` over structs.
* *Origin:* `Symbol: Ord` (mitigation §4) was the self-hosting form. The product meets the general one: **[V]** no std trait can be named as a bound at all, and a derived order is open (ADR-294 §6, ADR-204 §4).
* *Risk:* if this is decided *for the importer*, it breaks the CLAUDE.md rule. If it is not, the importer refuses most generic Rust.
* *Recommendation:* the importer refuses with a code. The language question gets its own record and is argued from what Nikaia is for.

**D. A stable `HashMap` lowering** (mitigation §3) - **decided** (October 11, 2026): [ADR-010](specification/adr/adr-010.md) D9, Part III 17.1, issue #581.
* One hasher per class of maps a value passes between, decided after the whole package is read; a map in a `pub` signature or type is a hidden type parameter each use fills (`nikaia_std::hash::Fast` or `Keyed`, default `Keyed`).
* For the importer: Rust `HashMap<K, V>` imports as `collections::HashMap[K, V]` (P: the hasher follows the keys' trust, so iteration order may change; Rust's `RandomState` order already changed from run to run, so a test that depends on it was not deterministic before). `HashMap<K, V, S>` with a hasher of the crate's choice drops `S` (P) until `hasher:` ([ADR-316](specification/adr/adr-316.md) D4) is built. A `pub` function or type of the round trip gains the hidden parameter; with its default it is the original's `HashMap<K, V>`, so the API diff shows no change for callers that name none.

**E. Explicit result sources `-> ref(world) String`** (mitigation §1b) - **rejected** in [ADR-283](specification/adr/adr-283.md) D25: a lifetime with a parameter's name, for an answer the body gives. The ledger keeps the spelling.

**F. Messages that must name Rust.**
* Part III C.2 rule 1: *"lifetime" … and Rust error codes never appear*. The importer's refusals are about Rust source and have to say `'a`, `dyn` and `unsafe`.
* *Recommendation:* C.2 gains one sentence. A diagnostic about Rust source names the Rust construct, and says what to write in Nikaia.

**G. The code range.** `NK28xx`, *importing from Rust*, in C.3, with the R rows of §1 as the first codes. Each code needs a reproduction test (C.3).

**H. The behaviour baseline.** The claim is *"the same as the original built with overflow checks on"*. The Rust release build's wrapping is not part of the claim. *Facts:* **[V]** the probes in §1.
* *Recommendation:* yes. The report flags any test that passes only by wrapping.

**I. Decoupling.**
* *Facts:* the move is at 61.0 %, with 30,003 Rust lines left in `crates/nikaia/src` **[V]**. A hand audit found the move's lines 53 % mechanical, 46 % adaptation and 1 % new design. What remains is the C-6/E9 cycles, where the work is host-trait design, not syntax. rs2nika saved about one turn per unit (C-5b).
* *Risk:* the product's quality bar (zero silent changes, L3 harnesses) would slow every self-hosting unit.
* *Recommendation:* decouple. Self-hosting keeps the draft script (steps 1-6 below). The product is a new issue and an ADR. They share only the corpus and the verification harness (Phase 1), which serves both.

### 5. Prior art (all **[M]** unless marked)

* **c2rust** (C → Rust): typed (it reads clang's AST), keeps behaviour by emitting `unsafe` Rust, and leaves the move to safe Rust to later refactoring tools. Its lesson for us: refuse or lower the fidelity claim rather than guess.
* **Go 1.5's C→Go translator** (cited in ADR-294 **[V]**): written for one codebase, near-perfect *because* it was tuned to that code's idioms. It argues for decoupling: a self-hosting tool can be perfect on its own corpus and still not general.
* **Kotlin's J2K** (Java → Kotlin in the IDE): uses the IDE's types and inserts `!!` where it cannot prove non-null. `.clone()` plays the same part here.
* **Swift and Objective-C:** interop was chosen over conversion. This argues for the partial-import route (§2).
* **Translation validation** (Pnueli 1998, Necula 2000): prove each run, not the translator. This is L4.

### 6. Phases, exit criteria, and how the six steps change

| Phase | What | Exit (measured) | *Perfect* means here |
| :--- | :--- | :--- | :--- |
| 0 | self-hosting, steps 1-6 | as step 6; tokens per moved line < 70k (`tokenspeed.md`, today 71k fresh-agent) | builder reviews |
| 1 | round-trip harness + fidelity table (shared) | runs on ≥ 5 gold commits, `naive.rs`, today's probes; reports the range bug as L2-pass/L3-fail | the report never over-claims |
| 2 | rules as a table, each E/P rule with a proof argument and a test | 0 functions L2-pass/L3-fail on all corpora; L2 share of draft lines on the 15 unmoved functions from 38 % to ≥ 60 % | no silent change |
| 3 | `nikaia import rust` preview: refusals with `NK28xx`, partial import | on an outside corpus fixed and predicted beforehand: per-level rates published, 0 silent | every refusal named |
| 4 | L4 normalised equality | share of functions at L4 published per corpus | proven where claimed |

**How the six steps change.**
* **Step 1:** extended. The corpus adds `benches/**` and the probes, and the metric adds L2/L3, not only `lower-std`.
* **Step 2:** constrained. A rule maps exactly, or it writes a marker naming the obligation. It never drops text or changes meaning (ranges, format arguments).
* **Step 3:** changed to decision A (a principle, not a measurement).
* **Steps 4-6:** unchanged for self-hosting. The product's stop rule is Phase 2's exit, not one turn per unit.

#### To build 1: the round-trip harness
* **Found:** by the 2026-10-11 design probe. A draft that passes `lower-std` can print a different number (`sum_to`: 55 against 45).
* **Today:** `rs2nika_passrate.py` stops at `lower-std`, and nothing runs a draft.
* **To build:**
  1. `scripts/rs2nika_roundtrip.py FILE.rs`. Per function: draft → `nikaia lower` → `cargo build` → run both binaries (original with `-C overflow-checks=on`) → print L0-L3 per function.
  2. Tests (a script run in CI): `sum_to` is reported **L2 pass / L3 fail**; `last_index` over an empty `Vec` is reported as an obligation, since the original panics or wraps and Nikaia gives `-1`; the hand-accepted Nikaia of 5 gold commits reaches L3 against the suite.
  3. Record the starting values in this issue.

#### To build 2: no silent rule
* **Found:** the same probe, and `naive.rs` (`{:.1}` lost its arguments).
* **Today:** R10 widens `i32` to `i64`, and there is no rule for ranges, and R15 drops what it cannot place.
* **To build:**
  1. Ranges: `a..b` → `a..<b`, and `a..=b` → `a..b`.
  2. Under `--profile none`, R10 maps only `usize`/`isize` (with an obligation note) and keeps `i32`/`u32`/`u64`.
  3. R15 refuses any format spec it cannot translate.
  4. Tests: `sum_to` reaches L3, and the `naive.rs` `report` line carries a marker. A `--lint` check fails on any `..` in a draft that came from Rust's `..`.

#### To build 3: the fidelity report and provisional refusal reasons
* **Found:** today's probe set: `NK1135` on 10 std bounds, `NK1158`, `NK1191`, `NK1209`, the parse errors on `dyn`, labels, tuple structs, `impl Iterator` and associated types.
* **Today:** the draft writes `// TODO(rs2nika)` or writes Nikaia that fails later.
* **To build:**
  1. `--report` prints the per-function table of §3, with an R reason from the table in §1 for every construct the script recognises and does not map. The names are provisional until decision G.
  2. Tests: one Rust snippet per R row produces that reason and no Nikaia for its function.
  3. File `println!` and `.iter().filter fn(x)` in `docs/lowering-defects.md` (C.1 defects, ADR-294 D3).

### 7. Risks, honestly

* **The idea argues against itself.** Nikaia has fewer features than Rust on purpose. **[V]** Today it cannot name `Ord`, `Hash`, `Display` or `Iterator` as bounds, and it has no `dyn`, no default bodies, no associated types, no operator overloading, no labels, no `ref mut` results and no macros. Most published crates use several of these **[M]**. So *perfect* can only mean *perfect on the subset that has an honest Nikaia, and a named refusal elsewhere*. A user will meet refusals often.
* **Pressure on the language:** every refusal invites a feature request "for the importer". CLAUDE.md forbids deciding the language that way (decisions C and E).
* **Idiom gap:** the faithful import of `naive.rs` is a `BTreeMap` of tuples. The Nikaia twin is a `grammar` plus `par_fold`, a redesign. Users may judge Nikaia by imported code.
* **The cost:** a full Rust grammar (decision A), a std call table (409 entries against Rust's std), differential harnesses, and the code range with its tests. This is months of work, not a slice. An estimate is **[U]** until Phase 1 measures.
* **API drift:** `usize`→`i64` changes the `pub` signatures, so the original tests need adapters.
* **Self-hosting slowed by the product:** avoided only by decision I.
