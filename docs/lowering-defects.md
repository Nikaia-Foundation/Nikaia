# Lowering defects met while moving Rust into Nikaia (#125, #558)

Checked on 2026-10-11 against `origin/develop` 707307a, with the binary built from it
(`/home/user/Nikaia-check/target/release/nikaia`). Every probe is a file in
`docs/lowering-defects/`; the NK2302 probes (`p/r2.nika`, `p/m.nika`, `p/m2.nika` ...) are in
`docs/lowering-defects/p/`. A user program is run with `nikaia run <f>.nika`; a `tools` probe is
copied into `crates/nikaia-std/src/tools/`, then `nikaia lower-std` and
`cargo check -p nikaia-std` are run (in a throwaway worktree).

Status: **verified** (reproduced today), **not reproduced** (the form named in the
commit messages works today), **by design** (a written rule, no defect).
`crates/rs2nika/convert/target/nikaia/bin/rs2nika --lint FILE.nika` (build it
with `nikaia build` in `crates/rs2nika/convert`) flags D1, D2, D3, D7 and D8 in
hand-written Nikaia before the 40 s `lower-std`; on today's `tools/*.nika` it
finds two D8 casts in `check_numbers4.nika`.

| # | Defect | Status | Probe |
| :--- | :--- | :--- | :--- |
| D1 | by-value read-only parameter lowers as `&T`; `p with {..}` writes `..p` | **built** (0.0.951: a `with` keeps its base, so the parameter stays a value) | d01_with_param.nika, probe_tools.nika |
| D2 | `m.get(k)?.clone() ?? d` clones a reference | **built** (a duplicate of D36; 0.0.965: re-checked over a `BTreeMap` of a struct and a `HashMap` of text) | `selfhosting_defects::a_cloned_get_over_either_map_gives_the_value` |
| D3 | `set.insert(..)` as an arm value: `bool` against `()` | not reproduced (0.0.951: runs) | d03_insert_tail.nika |
| D4 | a `for` over `Vec[Symbol]` yields `ref Symbol` | **built** (0.0.965: an element of a type that copies by declaration, the package's or a ledger `copies` one, binds the value) | `selfhosting_defects::a_loop_over_copies_binds_the_value` |
| D5 | `(A, B)?` does not parse | **superseded**: `(A, B)?` is written (ADR-364); to build in #648 | nullable.rs |
| D6 | `HashSet`/`HashMap` fields lower to `TrustedSet`/`TrustedMap` | **built** (0.0.840-0.0.848, #581: one trust analysis per package, one Rust type per class, a `pub` type's maps are hidden parameters defaulting to `Keyed`; 0.0.970: a Rust adapter names them, the order of the files changes nothing) | `map_trust.rs` |
| D7 | `x.len() as i64` writes `(x.len() as i64) as i64` | **built** (0.0.871: the cast is written once) | expressions.rs |
| D8 | `get(x.span.start as i64)` lowers to `&x.span.start as i64` | **built** (0.0.871: `&(x as i64)`) | expressions.rs |
| D9 | `Type?` parameter lowers to `Option<&Type>` | **closed**: not reproduced (0.0.965, by value and `ref`, `nikaia run`); D1 is built | `selfhosting_defects::an_optional_parameter_and_a_match_of_text_run` |
| D10 | `x ?? 0` on `Integer?` lowers to a plain integer | **built** (0.0.961: refused, `NK1103`; the fallback is of the left side's type, Part I 3.5) | probe_tools.nika |
| D11 | typed `throw` in a trait impl of a `throws` method is not boxed | **built** (0.0.951: raised into the box the signature declares) | d11_typed_throw.nika |
| D11b | a thrown `struct` without `impl Error` is not refused | **built** (0.0.961: `NK1161`, asked of the package's ledger, so an `impl` in another file counts) | d11b_free_throw.nika |
| D12 | function names are one namespace across `tools/*.nika` | by design (Part I 9.1), clear message | (none) |
| D13 | `grammar`/`test`/`throws` are reserved | by design, decided in #573; message names the word | kw_*.nika |
| D14 | a `Copy` value bound in a `match` inside a guard reaches its use as `&T` | **built** (0.0.946, #622: guards read copies) | d14b_guard_copy.nika |
| D15 | `BTreeMap[Symbol, V]` accepted by the checker, refused by rustc (`Symbol: Ord`) | **built** (0.0.850: `NK1247`, ledger `orders`) | ordered_keys.rs |
| D16 | `if match x {..} =>` in a guard does not parse; `if (match ..)` does | **built** (0.0.951: by design, ADR-301 D4; the refusal names the parentheses) | (rs2nika R14 writes the parentheses) |
| D17 | `s.starts_with(['a', 'b'])` passes the checker, lowers to `starts_with(vec![..])` (E0277) | **built** (0.0.887: refused, `NK1102`, #586) | nullable.rs |
| D18 | a two-parameter lambda over a list of tuples, `filter fn(name, ty) {..}`, lowers to `\|name, ty\|` (E0593) | **built** (0.0.961: refused, `NK1101`, help `fn(pair) { let (name, ty) = pair }`; whether it should destructure is #623) | `draft_a_root_is_one_of_two` |
| D19 | `match name { "i32" \| "i64" => .. }` with `name: ref String` lowers to `matches!(name, "i32" \| ..)` on `&String` (E0308) | **closed**: not reproduced (0.0.965: a `let`, a `return` and a guard; `a_copied_scalar` in `tools` now uses the or-pattern) | `selfhosting_defects::an_optional_parameter_and_a_match_of_text_run` |
| D20 | `x ?? s.len()` / `x ?? (0 - 1)` lowered to `|| s.len() as i64.into()` / `0 - 1.into()` | **built** (0.0.874) | expressions.rs |
| D21 | `a ?? 0 - 1` read as `a ?? 0` and a statement `-1` | **built** (0.0.874: refused, ADR-279 D1) | coalesce_fallback.rs |
| D22 | `cli::args().nth(at)` with `at: i64` lowered to Rust's `usize` `nth` (E0308) | **built** (0.0.875: `cli::Args`) | cli.rs |
| D23 | a `T?` as an operand (`if o != null { o + 1 }`, a guard `n if n > 3` over an `i64?` scrutinee) or an `f"{o}"` hole passed the checker, rustc E0369/E0277 | **built** (0.0.887: refused, `NK1102`, #586) | nullable.rs |
| D24 | `for b in text.bytes() { …; break }` lowered as written, clippy `never_loop` (deny) | **built** (0.0.887: allowed on `tools`, lib.rs, #586) | lib.rs |
| D25 | `impl Fatal for T` without `impl Error for T` not refused (ADR-345 D1) | **built** (0.0.887: `NK1164`, #586) | process_exit.rs |
| D26 | `mut e: ref E` returned: lowered `fn f(e: &mut &E) -> &E`, rustc "missing lifetime" | **built** (0.0.961: refused, `NK1131`; a `mut` view handed back is a view of the caller's view) | gaps p25 |
| D37 | a `ref` field of a struct literal from a part of a loop variable takes no `&` | **built** (0.0.961: checker fact `lent_fields`) | `check_core_flow.nika` |
| D39 | `One { n: -2 }.go() catch { 0 }`: a struct literal heads a `match` scrutinee (rustc: struct literals are not allowed here) | **built** (0.0.961: written in brackets) | `a_struct_literal_before_a_catch_runs` |
| D40 | `fn take(s: String) { let mut t = s; t.push_str("!") }` lowered `s` as `&str` (rustc refuses `push_str`) | **built** (0.0.961: a `let mut` of a parameter keeps it, `keeps.nika`) | `a_parameter_moved_into_a_let_mut_stays_owned` |

| D41 | `strip_prefix(prefix)` with an owned `prefix` declared outside a loop is `NK2105` (passed to a method that keeps it) | **built** (0.0.965: every text method's pattern is only read, `check_core_method_pattern_at`; `throws.nika` drops its `prefix.clone()`) | `selfhosting_defects::a_pattern_handed_to_strip_in_a_loop_is_only_read` |
| D42 | a tools file whose function throws through `net`'s `io::IoError` without `use std::io` lowers to `Thrown<io::IoError>` with no `io` in scope (rustc E0433); a program has the prelude | open: write `use std::io` | `map_trust.rs` `HM_C` |
| D45 | `throw made()`, a call whose result is an error type, does not name that type in the error set: the function lowers to `Box<dyn Error>` where `throw Bad::No` gives `Thrown<Bad>` (a tools caller that `split()`s it fails, E0599) | open: throw the constructor (`build_eval.nika`, #636) | probe: `fn made() -> Bad`, `fn f() -> i64 throws { throw made() }` |
| D46 | `xs[i] = v` on an `Array[T, N]` does not build: `[i64; 5]: Set<_, _>` is not satisfied (`nikaia_std::index::set`); the interpreter used to compute such a `comptime` where the program could not run it | open: build with `push` on a `Vec` | `fn squares() -> Array[i64, 5] { let mut xs: Array[i64, 5] = [0, 0, 0, 0, 0]; for i in 0..<xs.len() { xs[i] = i * i }; return xs }` |
---

## D1. A by-value parameter that is only read lowers as `&T`, and `p with {..}` then writes `..p`

### Found
By the emit self-hosting sessions (E7/E9, `EmitFlow` is passed by value and derived with `with`). Worked around by writing derivations as `self` methods, or the struct literal in full.

### Today
```nika
struct Flow { a: bool, b: bool }
fn flipped(f: Flow) -> Flow sync {
    return f with { a: !f.a }
}
```
Expected: `fn flipped(f: Flow) -> Flow { Flow { a: !f.a, ..f } }`, or with `&Flow`, `..*f` / `..f.clone()`.
Actual: `fn flipped(f: &Flow) -> Flow { Flow { a: !f.a, ..f } }`, which gives rustc E0308 (expected `Flow`, found `&Flow`). The same with a tools struct (`probe_with(f: EmitFlow)` gives `EmitFlow { statement: 1, ..f }` with `f: &EmitFlow`).
Workaround (verified): `return Flow { a: !f.a, b: f.b }`.

### To build
1. A test: a `with` over a by-value parameter that is only read, for a `Copy` struct and for one with a `String` field. Each runs.
2. The lowering writes the base of `..` as the place it has. For a parameter lowered as `&T`, that is `..*p` for a `Copy` type and `..p.clone()` otherwise, or the parameter stays by value when a `with` reads it whole.
3. Remove the `self`-method workaround the emit modules took for it, if any is only that.

## D2. `m.get(k)?.clone() ?? d` on a map clones a reference

**Built** as D36 (a duplicate). Re-checked in 0.0.965 over a `BTreeMap` of a struct and a `HashMap` of `String`; the D36 loops in `check_core_flow.nika` and `check_core_places.nika` are gone.

### Found
By the check self-hosting session (b3bb1c9), and again by the rs2nika draft of `pattern_type` (2026-10-11).

### Today
```nika
fn lookup(m: ref collections::BTreeMap[String, Ty], k: ref String) -> Ty sync {
    return m.get(k)?.clone() ?? Ty { name: "unknown" }
}
```
Actual: `match m.get(k).as_ref() { Some(__nikaia_it) => Some(__nikaia_it.clone()), .. }`. Here `__nikaia_it` is `&&Ty`, so `.clone()` gives `&Ty`, and rustc reports E0308 twice. In `d08` it also adds `String: Copy` (E0277).
Expected: the `?.clone()` of an `Option<&T>` gives `Option<T>`, as `m[k]?.clone()` does.
Workaround (verified): `m[k]?.clone() ?? d`.

### To build
1. A test: `get(k)?.clone() ?? d` over a `BTreeMap` and a `HashMap`, for a struct value and for `String`.
2. The lowering of `?.` on a `get` result does not add `.as_ref()` to an `Option` that already holds a reference (the index form's path), so `.clone()` is applied to `&T`.

## D3. `set.insert(..)` as the value of a `match` arm or `if` branch: `bool` against `()`

### Today
```nika
fn note(mut seen: collections::BTreeSet[String], k: i64) sync {
    match k {
        0 => seen.insert("zero"),
        else => seen.insert("other"),
    }
}
```
Actual: the arms keep `seen.insert(..)` as values in a function returning `()`, and rustc reports E0308 (expected `()`, found `bool`).
Expected: a `match` in statement position discards its arms' values, as Nikaia's checker already does (it accepts the program).
Workaround (verified): `0 => { let _added = seen.insert("zero") },`.

### To build
1. A test: `insert`, `remove` and `push` as the last call of an arm and of an `if` branch in statement position.
2. The emitter writes `;` after an arm's or branch's value when the `match`/`if` is a statement, or wraps it in `{ ..; }`.

## D4. A `for` over a `Vec[Symbol]` binds `ref Symbol`, which is not a `Symbol`

**Built (0.0.965).** A loop variable lent an element of a type that copies by declaration (the package's own that derives `Copy`, or a described type whose ledger says `copies`) binds the value: the checker asks `a_view_that_copies` (`check_core_stmt.nika`), and the lowering reads it once where the body opens (`let n = *n;`), as for a view of a number.

### Today
```nika
for n in names {          // names: ref Vec[winnow_grammar::Symbol]
    m[n] = 1              // m: BTreeMap[Symbol, i64]
}
```
Actual: NK1105 `This key is ref winnow_grammar::Symbol, but the map's keys are winnow_grammar::Symbol`. `world.words.resolve(n)` is accepted, and the lowering writes `*n`.
Expected: the ledger says `copies = true` for `Symbol` (`described.contracts`), so a loop variable over a list of copies reads as the value, as for `i64`.
Workaround: `n.clone()` (a copy of the handle).

### To build
1. A test in `tools`: a loop over `ref Vec[Symbol]` that stores each name in a map and passes it to `resolve`.
2. A loop variable over a `ref Vec[T]` with `T` a copy type (ledger `copies`) has type `T`.

## D5. `(A, B)?` does not parse: by design, but the message does not name the way out

Built (0.0.913): the checker refuses it with `NK1254`, naming a struct and `Name?` or `(A?, B?)` (`types.nika`, test in `nullable.rs`).

### Today
`fn pair(n: i64) -> (i64, i64)? sync` is refused with `Expected { here, but found ?`. Part I 2.3 says `(A, B)?` is not written (ADR-278 D1), so the refusal is the rule. Only the message is short of it.

### To build
1. The parser's message after `)` + `?` in a type: "`(A, B)?` is not written: name the pair with a `struct`" (Part I 2.3).
2. A test of the message.

## D6. `HashSet`/`HashMap` fields: the lowered type depends on where the file builds them: **built**

### Today
The tools package is lowered with one trust analysis over all its files (`sysroot.rs` `lower_tools_files`), and a map has one Rust type per class (ADR-010 D9, Part III 17.1). A `pub` struct of the tools that holds maps lowers with one hidden parameter per map, `__S0: nikaia_std::hash::Seed = nikaia_std::hash::Keyed`: a Rust adapter that writes `HmNames` gets the keyed map, one that writes `HmNames<nikaia_std::hash::Fast, nikaia_std::hash::Fast>` the fast one (`a_rust_adapter_names_a_tools_struct_with_and_without_its_hashers`). The files read in another order lower to the same sections (`the_real_tools_package_lowers_the_same_read_backwards`).

What is left is #581's step 11: a `HashSet` in the tools is the **keyed** one today wherever its class leaves its function (measured: `CheckComparable`'s `seen` as a `HashSet` costs 580,527 more instructions lowering `ty.nika` than as a `BTreeSet`, +0.10 %), so the tools keep `BTreeMap`/`BTreeSet` until the fill per class at use sites (#581) makes such a class fast.

## D7. `x.len() as i64` writes a doubled cast

### Today
`return xs.len() as i64` lowers to `(xs.len() as i64) as i64`, and clippy's `unnecessary_cast` stops CI (`-D warnings`).
Built in 0.0.871: the emitter writes the length once (`emit_core_cast`); test `a_cast_of_a_length_to_i64_is_written_once`.

### To build
1. The checker warns that the cast does nothing (`len()` is `i64`), or the emitter drops an `as` to the type the operand already has.
2. A test under `cargo clippy -D warnings`.

## D8. A cast inside a map read lowers as `&x as i64`

### Today
```nika
return m.get(x.span.start as i64)?.clone() ?? "none"     // start: u32
```
Actual: `m.get(&x.span.start as i64)`, which gives rustc E0606 (casting `&u32` as `i64` is invalid), plus D2's two errors.
Expected: `m.get(&(x.span.start as i64))`.
Built in 0.0.871: a cast written after a `&` is bracketed; test `a_borrowed_cast_is_bracketed`. D2's errors remain.

### To build
1. The emitter brackets a cast before it borrows a key: `&(e as T)`.
2. A test with a `u32` field cast to an `i64` key, through `get` and `[]`.

## D9. `Type?` parameter: not reproduced as named

**Closed (0.0.965):** not reproduced again, by value and as `ref Ty?`, in a user program; D1 is built.

### Today
In a user program, `t: Ty?` lowers to `Option<Ty>` and runs, and `t: ref Ty?` lowers to `Option<&Ty>` (correct). In `tools`, a read-only `t: Ty?` lowers to `&Option<Ty>`, which is D1's rule, and compiles. If a commit met `Option<&Type>` against an owned slot, it was D1.
**To build:** nothing beyond D1, until a probe shows a failure.

## D10. `x ?? 0` over an `Integer?` (a struct) lowers to a plain integer

### Today
```nika
fn probe_integer(v: Integer?) -> i64 sync {
    let n = v ?? 0
```
The checker accepts it. The emitter writes `nikaia_std::index::or(v, || 0)`, and rustc reports E0308 (expected `Integer`, found integer).
Expected: NK1105-style refusal at the checker. `0` is not an `Integer`, and the fallback must have the left side's type.
Workaround: `v ?? integer(0)` (or an `Integer { .. }` literal).

**Built (0.0.961).** The checker refuses it with `NK1103`: a fallback of another type than a type of the program's own that the left side holds, a number written out among them (Part I 3.5 now says the fallback is of the left side's type). Test: `defects_found_running.rs` `a_number_beside_a_struct_option_is_refused`.

### To build (done)
1. The checker types `??`'s fallback against `T` of a `T?` whose `T` is a struct, as it does for text and numbers.
2. A test: a refusal for `struct?` + a number literal; a struct literal fallback runs.

## D11. A typed `throw` inside a trait impl is not boxed

### Today
```nika
trait Step { fn go(ref self) -> i64 sync throws }
impl Step for One {
    fn go(ref self) -> i64 sync throws {
        if self.n < 0 { throw Bad::Negative(self.n) }    // enum Bad + impl Error for Bad
```
Actual: the trait and impl are declared `Result<i64, Box<dyn std::error::Error>>`, but the throw writes `Err(nikaia_std::error::throwing(Bad::Negative(..), &"go"))` with the typed error, and rustc reports E0308.
Expected: an untyped-throws signature boxes what is thrown (`.into()` / `Box::new`).
Workaround: not verified. A free function whose error set is inferred does not have the boxed signature.

### To build
1. A test: a trait method that `throws`, implemented with a typed `throw` and with a call that rethrows, caught at the caller.
2. Where the enclosing signature is `Box<dyn Error>`, the throw boxes its value.

## D11b (new). A thrown struct without `impl Error` reaches rustc: **built** (0.0.961)

`NK1161` now also refuses a `struct` or `enum` the package declares with no `impl Error` in any of its files (the checker reads the package's ledger; checked with a two-file project, the `impl` in the other file is accepted). Test: `a_thrown_struct_without_impl_error_is_refused`.

### Today
`struct Bad { code: i64 }` … `throw Bad { code: 1 }` passes the checker, and rustc reports E0277 (`Bad: std::error::Error` not satisfied). Part I 7.1: "What is thrown implements `Error`, and the `impl` line says so". NK1161 refuses only numbers, `bool`, characters and text.

### To build
1. The checker refuses a `throw` of a declared type without `impl Error`, naming the `impl` line to write (a new NK code in Part III's table; it is a rule already written in Part I 7.1).
2. A test.

## D12. One namespace for all `tools/*.nika`: by design

`node_key` declared in a second tools file is refused: "`node_key` is declared twice in this package … All files of a package share their names." This is Part I 9.1. The convention stays (CLAUDE.md: prefix per module); rs2nika's `--prefix` writes it.

## D13. `grammar`/`test`/`throws`/`dsl`/`comptime` as names: by design

Refused as a field, a parameter and a function name. The note says "`test` is a reserved word, so it can't be used as a name here". Decided in #573 (stay reserved). Nothing to build.

## D14 (new). A `Copy` value bound in a `match` inside a guard reaches its use as `&T`

### Found
By the rs2nika draft of `pattern_type` (2026-10-11): `Pattern::Ref { name: called, .. } if (match ..)` passed `called` to `resolve()`.

### Today
```nika
Shape::Named { id, .. } if (match wanted { Shape::Named { size: other, .. } if small(other) => true, else => false }) => small(id),
```
Actual: the guard's inner `if small(other)` gets `other: &i64` (the `let other = *other` is written in the arm body only), and rustc reports E0308.
Expected: as #578 asks for nested matches, a `Copy` payload bound under a reference reads as the value, here in a guard.
Workaround: a helper function that takes the matched value by `ref` (`draft_a_class_run`, `bare_reference` in `check_grammar.nika`).

### To build
1. Add the guard case to #578's test (a `Copy` payload bound in a guard's nested `match` and used there).
2. The deref the arm body gets is written in the guard too.

## D15 (new). `BTreeMap[Symbol, V]` passes the checker and fails in rustc

### Today
`Symbol` is `Ord` since winnow-grammar b27d8ba (Nikaia's `rev` has it), so this key now compiles; the refusal for a key with no order is still owed.
With `n.clone()` as the key, `collections::BTreeMap[winnow_grammar::Symbol, i64]` is accepted by the checker. rustc reports E0277 (`Symbol: Ord` not satisfied). `HashMap[Symbol, V]` compiles. The ledger has no claim for an order (`compares`, `copies`, `crosses` only).

### To build
1. Either the checker refuses an ordered map keyed by a type with no order (a refusal that names `HashMap`, or a key by text), or `Symbol` gets an order (see the mitigation table: owner).
2. A test.

## D16 (new, small). A `match` as a guard or condition needs brackets

`X if match y { .. } => ..` does not parse ("Expected `!`, `-`, `false` or `true`"), and `X if (match y { .. }) => ..` does. The fix is either the parser accepting it or the message naming the brackets. rs2nika's `matches!` rule writes the brackets.

## D17-D19 (new). Accepted by the checker, refused by rustc

Found by running rs2nika's drafts of unmoved `check/mod.rs` functions through
`lower-std` and `cargo check` (2026-10-11): 9 drafts passed the checker, and 3 of them failed in rustc.
* D17: `ty.starts_with(['i', 'a'])` (a list of characters) is accepted, and it lowers
  to `ty.starts_with(vec!['i', 'a'])`. A `Vec<char>` is not a `Pattern` (E0277). Either
  the checker refuses a list there, or the emitter writes `&['i', 'a'][..]`.
* D18: `pairs.iter().filter fn(name, ty) { .. }` over `Vec[(String, Ty)]` is accepted, and it
  lowers to `|name, ty|`. rustc wants one tuple argument (E0593). **Built (0.0.961)**: refused
  with `NK1101`, the help writing `fn(pair) { let (name, ty) = pair … }`. Whether the short
  form should take the tuple apart is a language question: #623 (`decision`).
* D19: a `match` of string literals over `name: ref String` (in a guard or a
  `matches!`) lowers without `.as_str()` (E0308).
* D19 **closed (0.0.965)**: not reproduced in a `let`, a `return` or a guard; the test is `selfhosting_defects::an_optional_parameter_and_a_match_of_text_run`.
**To build:** each a test through `nikaia run`, and the lowering fix named above,
or a checker refusal that names the spelling that works.

## D20 (new). A let-bound lambda called with an index argument: **built** (0.0.895)

A call of a let-bound function value lends what its type takes as a view, as a
call of a named function does (`views_kept.rs`).

## D27 (new, language). No rest pattern `V(..)`: **built** (0.0.920)

Rust's `Held::Precondition(..)` had no Nikaia spelling (#579, `claims_report`).
Decided in [ADR-349](specification/adr/adr-349.md): `E::V(..)` matches all of a
variant's values and lowers to Rust's `E::V(..)`, so a draft passes it through
unchanged (`match_patterns.rs`). A positional rest `E::V(a, ..)` is `NK1255`.

## D28 (new, small). An owned `String` passed to `contains`: **built** (0.0.895)

Owned text handed to `starts_with`, `ends_with` or `contains` is lent, as to any
`ref String` parameter, and stays the caller's (`views_kept.rs`).

## D29 (new). `tools/*.nika` checked as a project: **built** (0.0.898)

The cause: a project's own `sync` inference (`modules::Program::of`) read `std`
and the dependency packages, not the described crates' `contracts/*.contracts`
the check reads, so a function calling only a described `sync` entry
(`InternerContext::resolve`) was inferred to pause. It reads them now
(`described_entries.rs`).

## D30 (new). D28 for `starts_with`, and `match` on a `?? continue` binding: **built** (0.0.904)

A local f-string handed to `starts_with` is lent since D28. A `let` that binds
what `get` hands back past a jump (`let held = m.get(k) ?? continue`) is a
reference below, so a `match held` binds by reference by itself and writes no
`ref` (`views_kept.rs`).

## D31 (new). `list[i].field ?? continue` moves out of the element: **built** (0.0.904)

An option read out of a list element before a jumping fallback is cloned out of
the element (`.clone()` before the `match`) unless the coalesce lends it
(`views_kept.rs`).

## D32 (new). `xs.iter().find_map(..) ?? "lit"` ambiguous in rustc: **built** (0.0.904)

An iterator's answer (`find_map`, `find`, `next`, `last`, `nth`, `max`, `min`)
beside a literal fallback could be `Option<&String>` or `Option<String>`, and
`"lit".into()` matched both `Or` impls. The answer is mapped to text of its
own first (`.map(|v| v.to_string())`), so a typed `let x: T? = ..` is no
longer needed (`views_kept.rs`).

## D33. A tuple-variant pattern with too few fields passes the checker: built

Built: the checker refuses it with `NK1253` (`check_members.nika`, test in `crates/nikaia/tests/match_patterns.rs`).


`match e { E::A(_) => 1, else => 0 }` with `A(i64, i64)` passes `nikaia check`;
rustc refuses the lowering (*this pattern has 1 field, but the corresponding
tuple variant has 2 fields*). The checker should name the arity itself (a
`NK` refusal), though D27's `V(..)` now covers the whole payload. Found with rs2nika
(#579) while trying `(_)` for `(..)`.

## D34. A by-value enum parameter, and a `match` arm that is a call returning a value: arm **built**, parameter **not reproduced**

Built: an arm of a statement `match` is lowered as `{ expr; }` (`emit_core_match.nika`, test in `match_patterns.rs`). The parameter half did not reproduce on 0.0.908 (`fn f(op: BinaryOp)` called with a pattern-bound `op`, free function and method in a tools file, lowers `op: BinaryOp` and `f(*op)`); it needs the exact function that failed.


`fn f(op: BinaryOp)` lowers its parameter as `&BinaryOp`, while a call with an
`op` bound by a `match` pattern writes `f(*op)`: E0308. And a statement `match`
whose arm is `map.insert(k, v)` (an `Option`) lowers the arm without `;`, so
the arms disagree with `_ => { }`. Workaround: the test inline, and a block
arm. Met in U-c1 (`check_core_leaves.nika`).

## D35 (new). A field option of a `get(..) ??` local moves out of the reference: **built** (0.0.913)

Built: a local bound from `get(..) ??` past a jump counts as lent, so `c.f ?? return` opens the field (`.as_ref()` / `.as_deref()`) as over a `ref` parameter (`check/mod.rs`, test in `views_kept.rs`).

`let c = map.get(k) ?? return x` then `let s = c.signature ?? return x` lowers
the second to `match c.signature { Some(v) => v, .. }`: E0507, *cannot move
out of `c.signature` behind a shared reference*. The same read on a `ref`
parameter lends (`match &c.signature`), so the lowering does not know that a
local bound from `get(..) ??` is a reference. Workaround: a helper function
over `ref T`. Met in U-c2 (`check_core_comptime.nika`).

## D36. `map.get(k)?.clone() ?? fallback` clones the reference: **built**

`args.push(bound.get(p)?.clone() ?? Ty::Unknown)` over a `BTreeMap[String, Ty]`
lowers to `match bound.get(p).as_ref() { Some(it) => Some(it.clone()), .. }`:
`it` is a `&&Ty`, its clone a `&Ty`, and `rustc` answers *expected `Ty`, found
`&Ty`*. It is the form NK1185's help offers (*`held?.clone() ?? …`*). The
`.as_ref()` is one level too many for a `get`, which already answers an
`Option<&T>`. Workaround: a loop over the map, `for (k, v) in m { if k == p {
return v.clone() } }`. Met in U-c9 (`check_core_places.nika`).
Built: a `?.` reach on a `get`, `first` or `last` takes no `.as_ref()`
(`emit_place_already_lent`, `emit_core_places.nika`); test
`views_kept::a_cloned_get_with_a_fallback_runs`.

## D37. A `ref` field of a struct literal from a part of a loop variable takes no `&`: **built** (0.0.961)

The checker records a value of its own given to a field that holds a view (or a view's `T?`)
in `lent_fields`, the counterpart of `lent_args`, and the emitter writes the `&`. The
workaround `check_core_flow_answered` is gone.

`answered.push(CheckArm { tail: arm.body, ty: t })`, with `tail: ref Expr?` and
`arm` a loop variable over a `ref Vec[MatchArm]`, lowers to `CheckArm { tail:
Some(arm.body), .. }`: `rustc` answers *expected `&Expr`, found `Expr`*. A
parameter `body: ref Expr` handed in the same place lowers as `Some(body)` and
compiles. Workaround: a function that builds the struct from a `ref`
parameter (`check_core_flow_answered`). Met in U-c8 (`check_core_flow.nika`).

## D38. `strip_prefix(f"...")` lowers to a `String` pattern: **built** (0.0.949, 0.0.950)

`line.strip_prefix(f"{key}=")` reached `rustc` as a `String` pattern (E0277).
The f-string is lent since 0.0.949 (#614), an owned `String` since 0.0.950;
`an_owned_pattern_to_strip_prefix_is_lent` runs both.

## Language questions filed as issues

Forms met here that need a decision, not a lowering fix; the issue holds the case.
* Nested tuple patterns in a `for` head, `for (a, (b, c)) in xs`: [#611](https://github.com/Nikaia-Foundation/Nikaia/issues/611).
* `null` as a pattern in a `match` over a `T?`: [#612](https://github.com/Nikaia-Foundation/Nikaia/issues/612).
* An element read from a borrowed list as a view, `let y: ref Expr = given[0]`: [#613](https://github.com/Nikaia-Foundation/Nikaia/issues/613).
* A result that views one of two `ref` parameters (`NK2302`, the `p/` probes above): [#614](https://github.com/Nikaia-Foundation/Nikaia/issues/614).

## `EmitHost::names_read` does not look into statements (#622): **built** (0.0.951: it walks the statements of blocks; the copy a body does not read is not written)

`names_read` (crates/nikaia/src/emit/host.rs) walks with `contracts::sync::visit_expr`, which does not enter the statements of a block. Filtering an arm's copied bindings by it dropped copies the body needed (all of `package.rs`), so a copy the arm body may not read is written `#[allow(unused_variables)] let a = *a;` instead (`emit_core_match.nika`). Fix: a walk over blocks too, then drop the copy instead of allowing it. Test: `match_patterns.rs` `a_kept_element_is_taken_and_a_guard_reads_a_copy`.

## D39 (new). A struct literal at the head of a `catch`: **built** (0.0.961)

`let a = One { n: -2 }.go() catch { 0 }` lowered to `match One { n: -2 }.go() { .. }`, and
rustc refused it: *struct literals are not allowed here*. The guarded half of a `catch` whose
head is a struct literal (or a `with`), through method calls and fields, is now written in
brackets. Test: `defects_found_running.rs` `a_struct_literal_before_a_catch_runs`.

## D40 (new). A parameter bound by a `let mut`: **built** (0.0.961)

`let mut t = s` and a change of `t` lowered `s: String` as a view. A `let mut` changes what it
binds, so `keeps` now counts a parameter bound by one as kept (owned), unless the author lent it.

## D26 note (0.0.961)

`mut e: ref E` is a view the caller's view is changed through (Part I 6.6: a `mut` parameter is
passed as `ref mut`), so handing it back would be a view of that view. It is refused with
`NK1131`, whose help takes `e: ref E` without `mut` and walks a local (`let mut at = e`). Test:
`a_mut_view_parameter_handed_back_is_refused`.

## D41 (new). A text method's pattern in a loop is taken: **built** (0.0.965)

`error.strip_prefix(prefix)` inside a `for`, with `prefix` an `f"..."` declared before it, was refused with `NK2105`: only `starts_with`, `ends_with` and `contains` counted their pattern as read (D28), so `strip_prefix`'s was handed over. Met in `throws.nika` (0.0.963), worked around with `prefix.clone()`. Built: the argument a text method takes as its pattern (`check_core_method_pattern_at`, #617) is lent, for every such method (`check_arguments.nika`); the clone is gone.

## D42 (new). `let x = m[k] ?? return` over a map of words keeps a `&T`

In `same_tiers` (`text_tiers.nika`), `let other = b[word] ?? return false` over a
`BTreeMap[String, Tier]` (`Tier` an enum of bare variants) lowered `other` as `&Tier`, and
handing it to `tier_word(tier: Tier)` failed in Rust (`expected Tier, found &Tier`). The checker
accepts it: a word is copied at a read (ADR-279 D8). Met when `NK1257` took `ref` off the
parameter (#628). Worked around with `let other: Tier = ...`, which lowers `.copied()`.

### To build

A `let` without a type over an index fallback whose element is a word lowers the copy, as the
typed one does; then remove the annotation in `same_tiers`.

## D43 (new). `f(m[k] ?? continue)` over a map of a struct hands `&&T` to a `ref` parameter

In `prover_claim_ran` (`prover_walk.nika`, #633), `written_of(walk.state.call_sites[call] ??
continue)` with `call_sites: BTreeMap[String, Expr]` and `written_of: fn(ref Expr) -> String`
lowered to `written_of(&match *get(..) { Some(v) => v, None => continue })`: `v` is already
`&Expr`, and rustc refuses the extra `&` (`expected Expr, found &Expr`). The checker accepts it.
Written as `let site = m[k] ?? continue` and `written_of(site)`, which lowers right.

### To build

An argument that is an index fallback over a map whose value is not a word passes the reference
the fallback already is, as a `let` of it does; then fold the two lines in `prover_claim_ran`
back into one.

## D44 (new). A function value kept with a `ref` parameter and a type that may pause: rustc "lifetime may not live long enough"

`struct One { f: fn(ref P, i64) -> i64 }` and `One { f: fn(q, n) { q.add(n) } }` pass the
checker. The kept type lowers to `Kept<dyn Fn(&P, i64) -> Boxed<i64>>`, and the closure to
`move |q, n| Box::pin(async move { q.add(n) }) as Boxed<_>`: the future borrows `q`, and
`Boxed` is `'static`, so rustc refuses it. A lambda, a named function and a method named by its
path (`One { f: P::add }`, #601) all meet it; with `sync` in the type, or a parameter taken by
value, they run. Met while building #601; its tests use `sync` kept types for methods that
take `ref self`.

### To build

A kept function type that may pause and takes a `ref` parameter lowers to a future that may
borrow it (`dyn for<'a> Fn(&'a P, i64) -> Pin<Box<dyn Future<Output = i64> + 'a>>`), or the
checker refuses it with a message that names `sync` or a parameter by value; then a test in
`function_values.rs` with `fn(ref P, i64) -> i64` and a method that pauses.

## D47 (new). `op ?? BinaryOp::Mul` over a `ref BinaryOp?` hands rustc `&BinaryOp` against `BinaryOp`

In a `match` on `Stmt::Assign { op, .. }` borrowed from a `ref Stmt`, `match op ?? BinaryOp::Mul {
BinaryOp::Add => …, else => … }` passes the checker and lowers to `match nikaia_std::index::or(op,
|| &BinaryOp::Mul) { … }`, which rustc refuses (`expected BinaryOp, found &BinaryOp`). `op.clone()
?? BinaryOp::Mul` is `NK1125`. Met while building #640: `prover_loops.nika` reads the operator in a
function of its own, `loop_assigned(arena, op, old, value)`, with `let operation = op ?? return
value`, as `bounding_compound` in `bounds_walk.nika` does, which lowers right.

### To build

A fallback that is a variant literal, after an option of a borrowed enum, lowers to the same
reference on both sides (or both by value); then fold `loop_assigned` back into its one caller,
with a test in `lowering.rs`.
