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
| D1 | by-value read-only parameter lowers as `&T`; `p with {..}` writes `..p` | verified | d01_with_param.nika, probe_tools.nika |
| D2 | `m.get(k)?.clone() ?? d` clones a reference | verified | d02_get_clone.nika |
| D3 | `set.insert(..)` as an arm value: `bool` against `()` | verified | d03_insert_tail.nika |
| D4 | a `for` over `Vec[Symbol]` yields `ref Symbol` | verified (as a checker type) | probe_symkey.nika |
| D5 | `(A, B)?` does not parse | by design; message does not name the way out | d05_tuple_opt.nika |
| D6 | `HashSet`/`HashMap` fields lower to `TrustedSet`/`TrustedMap` | verified: the type follows each file's own trust analysis; decided in ADR-010 D9, build #581 | probe_hash.nika |
| D7 | `x.len() as i64` writes `(x.len() as i64) as i64` | **built** (0.0.871: the cast is written once) | expressions.rs |
| D8 | `get(x.span.start as i64)` lowers to `&x.span.start as i64` | **built** (0.0.871: `&(x as i64)`) | expressions.rs |
| D9 | `Type?` parameter lowers to `Option<&Type>` | not reproduced as named; same root cause as D1 (`&Option<T>`) | d09_opt_param.nika, probe_tools.nika |
| D10 | `x ?? 0` on `Integer?` lowers to a plain integer | verified; also not refused by the checker | probe_tools.nika |
| D11 | typed `throw` in a trait impl of a `throws` method is not boxed | verified | d11_typed_throw.nika |
| D11b | a thrown `struct` without `impl Error` is not refused | verified (new) | d11b_free_throw.nika |
| D12 | function names are one namespace across `tools/*.nika` | by design (Part I 9.1), clear message | (none) |
| D13 | `grammar`/`test`/`throws` are reserved | by design, decided in #573; message names the word | kw_*.nika |
| D14 | a `Copy` value bound in a `match` inside a guard reaches its use as `&T` | verified (new; #578's family) | d14b_guard_copy.nika |
| D15 | `BTreeMap[Symbol, V]` accepted by the checker, refused by rustc (`Symbol: Ord`) | **built** (0.0.850: `NK1247`, ledger `orders`) | ordered_keys.rs |
| D16 | `if match x {..} =>` in a guard does not parse; `if (match ..)` does | verified (new, small) | (rs2nika R14 writes the parentheses) |
| D17 | `s.starts_with(['a', 'b'])` passes the checker, lowers to `starts_with(vec![..])` (E0277) | **built** (0.0.887: refused, `NK1102`, #586) | nullable.rs |
| D18 | a two-parameter lambda over a list of tuples, `filter fn(name, ty) {..}`, lowers to `\|name, ty\|` (E0593) | verified (new) | `draft_a_root_is_one_of_two` |
| D19 | `match name { "i32" \| "i64" => .. }` with `name: ref String` lowers to `matches!(name, "i32" \| ..)` on `&String` (E0308) | not reproduced in a user program (0.0.887, `nikaia run`) | `draft_a_copied_part` |
| D20 | `x ?? s.len()` / `x ?? (0 - 1)` lowered to `|| s.len() as i64.into()` / `0 - 1.into()` | **built** (0.0.874) | expressions.rs |
| D21 | `a ?? 0 - 1` read as `a ?? 0` and a statement `-1` | **built** (0.0.874: refused, ADR-279 D1) | coalesce_fallback.rs |
| D22 | `cli::args().nth(at)` with `at: i64` lowered to Rust's `usize` `nth` (E0308) | **built** (0.0.875: `cli::Args`) | cli.rs |
| D23 | a `T?` as an operand (`if o != null { o + 1 }`, a guard `n if n > 3` over an `i64?` scrutinee) or an `f"{o}"` hole passed the checker, rustc E0369/E0277 | **built** (0.0.887: refused, `NK1102`, #586) | nullable.rs |
| D24 | `for b in text.bytes() { …; break }` lowered as written, clippy `never_loop` (deny) | **built** (0.0.887: allowed on `tools`, lib.rs, #586) | lib.rs |
| D25 | `impl Fatal for T` without `impl Error for T` not refused (ADR-345 D1) | **built** (0.0.887: `NK1164`, #586) | process_exit.rs |
| D26 | `mut e: ref E` returned: lowered `fn f(e: &mut &E) -> &E`, rustc "missing lifetime" | verified (#586) | gaps p25 |

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

### Today
`fn pair(n: i64) -> (i64, i64)? sync` is refused with `Expected { here, but found ?`. Part I 2.3 says `(A, B)?` is not written (ADR-278 D1), so the refusal is the rule. Only the message is short of it.

### To build
1. The parser's message after `)` + `?` in a type: "`(A, B)?` is not written: name the pair with a `struct`" (Part I 2.3).
2. A test of the message.

## D6. `HashSet`/`HashMap` fields: the lowered type depends on where the file builds them

### Today
In `tools`, a struct with `seen: collections::HashSet[String]` lowers to `collections::HashSet<String>` when nothing in the file builds one. Once the same file writes `collections::HashSet()`, the field lowers to `collections::TrustedSet<String>` (the provenance rule, ADR-010 D5). The Rust adapter that builds or reads the struct then needs a type it cannot predict. In a user program this is the intended rule.
Workaround (the plan's): `BTreeSet`/`BTreeMap`.

**The mechanism, re-checked:** `tools` is lowered with each file's own trust analysis (`sysroot.rs` `lower_tools_at`). A method call on a value whose type another file declares is unresolved there and counts as untrusted, so that file writes `HashMap::new()` into a field its declaring file wrote as `TrustedMap` (rustc E0308). A project uses the entry file's analysis for all files.

### To build
Decided: [ADR-010](specification/adr/adr-010.md) D9, Part III 17.1. The build order and tests are #581.

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

### To build
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

## D11b (new). A thrown struct without `impl Error` reaches rustc

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
  lowers to `|name, ty|`. rustc wants one tuple argument (E0593). Either the checker
  refuses two names for one tuple, or the emitter writes `|(name, ty)|`.
* D19: a `match` of string literals over `name: ref String` (in a guard or a
  `matches!`) lowers without `.as_str()` (E0308).
**To build:** each a test through `nikaia run`, and the lowering fix named above,
or a checker refusal that names the spelling that works.

## D20 (new). A let-bound lambda called with an index argument fails in rustc

`let f: fn(ref String, i64) -> i64 sync = fn(a, b) { a.len() + b }` then
`f(xs[0], 1)` passes `nikaia check` and fails in rustc ("Mismatched types,
consider borrowing"): the call does not borrow `xs[0]` for the `ref String`
parameter, as a call of a named function does. Found with rs2nika's R62
(#579). **To build:** a `nikaia run` test, and the borrow at the call.

