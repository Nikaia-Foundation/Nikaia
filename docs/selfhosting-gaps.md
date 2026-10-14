# Why so little of the rs2nika draft survives: tool gaps and language gaps

A note for #125 and #579. Of a draft by `crates/rs2nika/convert`, 22-35 % of
the lines reach the finished `.nika` file. This note takes the twelve
functions of three self-hosting commits, puts the Rust original, the draft
(`rs2nika --profile check --file`, today's converter) and the finished Nikaia
side by side, and sorts every hand correction into one of five kinds:

* **TOOL**: Nikaia has the form, the converter does not write it;
* **LANGUAGE**: Nikaia has no direct form, so the code was restructured by hand;
* **COMPILER**: the form is decided and the checker takes it, the lowering breaks
  it (a defect, not a gap; they belong in `docs/lowering-defects.md`);
* **ADAPTER**: the code stays in Rust for a good reason (Rust types);
* **STYLE**: the deviation was not needed.

Every LANGUAGE guess was tried with `target/release/nikaia run` on a minimal
program (built from `develop` at the time of writing). What the
specification already decides is listed with its place, not as a gap.

**Material.** `5755b65` (`several_errors`, `wholly_known`, `a_map_type`,
`is_a_literal_other_than`, `a_copied_part`), `3e26691` (`tail_of`, `articles`,
`unpromised`), `07c5b14` (`payload_of`, `claims_report`, `byte_literal`,
`peeled`, `an_or_a`). 46 deviations between draft and finished code.

## Counts

| kind | deviations | share |
| :--- | ---: | ---: |
| TOOL | 19 | 41 % |
| LANGUAGE (`std` entries) | 11 | 24 % |
| STYLE | 9 | 20 % |
| ADAPTER | 4 | 9 % |
| COMPILER | 3 | 7 % |

The main finding: **no hand correction in this material needed new syntax.**
The language gaps are all missing `std` entries on `Seq` and on text. The
largest single share is the converter writing TODOs for forms that Nikaia has
(or-patterns, literal fields under a guard, optional views) and not writing
`?.`/`??` for Rust's `Option` combinators. A missing `Seq` method also costs
more than its line: the finished code moves the loop into a helper function,
so one Rust line becomes 6-8 lines, which is most of the low survival rate in
`wholly_known`, `a_copied_part` and `payload_of`.

Re-measured after the converter took items 1-4, 7 and the f-string half of 5
below (code lines of the draft, comments left out, found verbatim in the
finished function; ten of the functions, `several_errors`, `a_map_type` and
`an_or_a` could not be cut automatically): 42 of 125 lines (34 %) before, 44 of
130 (34 %) after, with 22 -> 11 TODO lines on the reference inputs; the rate
barely moves because the finished code was written before the `Seq` methods and
`?.` chains the draft now uses, and TODO comments were never counted.

## TOOL (19)

| form | n | Rust | the draft | by hand |
| :--- | ---: | :--- | :--- | :--- |
| `Option` combinators (`map`, `and_then`, `is_some_and`, `map_or`) | 5 | `block.stmts.last().map(\|s\| &s.node)` | `.map(fn(s) {..}) ?? return null` + TODO | index arithmetic; `block.stmts.last()?.node` works (probe `q33`) |
| or-pattern over variants marked TODO | 3 | `Expr::LitStr { .. } \| Expr::LitInterpolated { .. } =>` | TODO "one arm per variant" | one arm each; the or-pattern works (Part I pattern table, probe `p02`) |
| literal field under a guard marked TODO | 2 | `Ty::Named { name, args, view: false } if name == "Error"` | TODO "bind the field" | `view } => if !view && ..`; the form works (probe `q06`) |
| `Some(x)` kept in a `T?` result | 2 | `=> Some(args)` | `Some(args)` | `args` |
| stale TODO "no optional view `ref T?`" | 2 | `-> Option<&Expr>` | TODO | `tail_value_of(..) -> ref Expr?` is in the finished code |
| f-string hole with a call, local lambda without a type | 2 | `let count = \|want: fn(&Held) -> bool\| ..` | 5 TODOs, `let count = fn(want) {..}` (`NK1245`) | counters in the loop; `let count: fn(i64) -> i64 = fn(want) {..}` works (probe `q17b`) |
| `String::with_capacity(n)` | 1 | `String::with_capacity(text.len() + 8)` | kept (`NK1171`) | `let mut out: String = ""` |
| `byte as char` | 1 | `byte as char` | `byte as scalar` | `(byte as u8) as scalar` |
| `Refused \| None` arm over an `Option` | 1 | `Some(Held::Refused) \| None =>` | `?? return` inside the loop (wrong: returns from the function) | a function of its own with `?? return` (the converter's own hint) |
| `--prefix` applied inside a string literal | 1 | `Some("Give \`process::exit\` ..")` | `"CheckGive .."` (wrong text) | the literal as written (`an_exit_code_out_of_range`) |
| iterator chain `.iter().filter(..).cloned().collect()` | 1 | `guarded.thrown.iter().filter(..).cloned().collect()` | `.filter(..)?.clone().collect()` | a `for` loop with `push` (`a_fatal_error_passes`) |
| `Option::as_mut()` on a field | 1 | `if let Some(outer) = self.s.flow.guarded.as_mut()` | `as_mut()` kept | clone, change, assign back (as `may_fail_here`) |
| `self.parsed.interner` | 1 | `&self.parsed.interner` | `world.interner` | `world.words` |
| `crate::ast::int_value` (`i128`) not in Nikaia | 1 | `int_value(*value, *negative)` | kept, listed as missing | compare the `u64` literal and its sign directly |
| COMPILER: `s.starts_with(p)` with `p` a local `let p = f"..."` | 1 | `taken.path.starts_with(&format!("{name}."))` | `NK1102` (*expects `ref String`, passing `String`*), also inside the loop; outside it `NK2105` | a helper taking `p: ref String`, called with the f-string (`compare_path_within`) |
| COMPILER: `match` on a binding from `map.get(k) ?? continue` | 1 | `match held { Ty::Named { name, args, .. } if .. }` | lowers `ref name` inside an implicitly borrowing pattern (rustc: *cannot explicitly borrow within an implicitly-borrowing pattern*) | a helper with a `ref Ty` parameter (`compare_parameter_bounds`) |
| LANGUAGE: `ref mut` parameter for a recursive walk's `seen` set | 1 | `seen: &mut BTreeSet<String>` | `NK1158` (*`ref mut` only for C*) | `mut seen: collections::BTreeSet[String]` |
| LANGUAGE: a method on `CheckState` handing back a struct of views of `self` and `world` | 1 | `Comparable { structs: &self.s.structs, own: self.world.own, .. }` | `NK2302`, and `self.structs` is not a `ref` field value (`NK1106`) | Rust builds `CheckComparable`; Nikaia takes it as `ref` in free functions |
| STYLE: `f"{name}{args:?}"` as a memo key | 1 | Debug of `Vec<Ty>` | not available | `name[` + `Ty::text` joined `]`, unique the same way |
| STYLE: `Vec::extend` with a `T?` | 1 | `refused.extend(found)` (Option is an iterator) | `NK1102` | `if found != null { push(found ?? continue) }` |
| COMPILER: `list[i].field ?? continue` on a non-copy field | 1 | `if let Some(refusal) = &mut slot.refusal` | E0507 in rustc (D31) | `slots[i].refusal?.clone() ?? continue` |
| LANGUAGE: a struct of views built from `self`'s fields inside a method | 1 | `Comparable { structs: &self.s.structs, .. }` in `declared` | `NK1106` (`state.structs` is not a `ref`) | the maps passed as `ref` parameters to a free function that builds it (`collect_keys_unordered`) |
| STYLE: `loop { .. }` | 1 | `loop` in `collect_copies` | `NK1117` | `while true` |
| STYLE: `match op { Add => .., Sub => .. }` returning a tuple | 1 | `let (value, heading) = match op {..}` | kept as `if` chain over `op ==` | an `if`/`else if` chain assigning two `let mut` |
| STYLE: `matches!(want, Ty::Nullable(_))` | 2 | `matches!` | none | `match want { Ty::Nullable(_) => true, else => false }` |

### For the converter, by frequency

1. Rewrite `Option` combinators to `?.` and `??`: `o.map(|x| x.f)` -> `o?.f`,
   `o.and_then(|x| g(x))` -> a `let` and `??`, `o.is_some_and(p)` ->
   `(o?.p() ?? false)`, `o.map_or(d, f)` -> `(o?.f() ?? d)`. Each `?.` step of
   a chain needs its own `?.` (probe `q31`, `NK1125` names it).
2. Drop the TODO for or-patterns over variants and for a literal field under a
   guard; both are Part I patterns and compile.
3. Drop the TODO "no optional view `ref T?`": `-> ref T?` compiles in
   `tools/*.nika` (`check_types::tail_value_of`).
4. Strip `Some(..)` in arms and returns of a `T?` function.
5. Give a lambda bound to a `let` its function type from the closure's
   parameter types; bind a call in an f-string hole to a `let` itself rather than
   leaving a TODO.
6. Never write `?? return` where the Rust arm is a value of the loop body: move
   the `match` into a function first, or write `match` with `null =>`.
7. `String::with_capacity(..)` -> `""` with a type; `x as char` from a byte ->
   `(x as u8) as scalar`.

## LANGUAGE (11): `std` entries, no syntax

| form | n | Rust | probe today | by hand |
| :--- | ---: | :--- | :--- | :--- |
| `Seq::all` / `Seq::any` | 5 | `parts.iter().all(wholly_known)` | `NK1171` (`p07`, `p08`) | a helper fn with a `for` and an early `return` |
| `Seq::find` / `Seq::find_map` | 3 | `items.iter().find_map(\|item\| match .. )` | `NK1171` (`p09`, `p10`) | nested `for` with `return` from a function of its own |
| `str::rsplit`, `Seq::last` | 1 | `name.rsplit("::").next().unwrap_or(name)` | `NK1171` (`p23`, `p24`) | a helper `type_last_segment` |
| `str::split_at` | 1 | `let (before, from) = rest.split_at(at)` | `NK1171` (`p22`) | the text as `Vec[scalar]` and index loops |
| `Seq::take_while` | 1 | `from[1..].chars().take_while(\|c\| *c != '`').collect()` | `NK1171` (`p11`) | a `while` over indices |

`Seq` has today `collect count nth next join map filter for_each rev zip take
skip step_by` (`std.contracts`). The draft writes `.filter(!p).count() == 0`
for `all`, which compiles but reads worse than the helper, so it was replaced.

### `Seq::any`, `Seq::all`

*Form:* `types.iter().all fn(t) { wholly_known(t) }`, a truth over the
elements that stops at the first answer. *Today:* `NK1171`. *Others:* Rust
`Iterator::any/all`, Kotlin `any/all/none`, Swift `contains(where:)` and
`allSatisfy`, Python `any()/all()` built-ins. Every one of them counts it as
basic. *Fit:* a pure, short-circuiting reduction over a `Seq`; it has the same
contract shape as `filter` (`fn(T) -> bool`), `sync` follows the lambda, and no
new type is involved. *First recommendation:* add both to `Seq` as `std`
entries. Nothing in the language argues against them.

### `Seq::find`, `Seq::find_map`, `Seq::position`

*Form:* `items.iter().find fn(i) { i.name == n }` -> `T?`; `find_map` with a
lambda returning `U?` -> `U?`. *Today:* `NK1171`; `filter(..).next()` works
(probe `q07`) and covers `find`. *Others:* Rust `find/find_map/position`,
Kotlin `find/firstNotNullOfOrNull/indexOfFirst`, Swift `first(where:)`,
`firstIndex(where:)`, `compactMap(..).first`, Python `next((x for ..), None)`.
*Fit:* `T?` is Nikaia's absent value, so `find` is the natural end of a `Seq`;
`find_map` is `filter_map` + `next`. A `filter_map` (`map` to `U?` and drop
`null`) is the more basic one, and Kotlin's `mapNotNull`/Swift's `compactMap`
show it is expected. *First recommendation:* `find`, `position` and
`filter_map`; `find_map` only if `filter_map(..).next()` does not lower to the
same loop. The draft would then survive in `payload_of` as written. Note: a
Rust `find_map` whose closure uses `?` stays a hand rewrite, because there is
no postfix `?` (ADR-023 D8, decided).

### `Seq::take_while`, `Seq::skip_while`

*Form:* `s.chars().take_while fn(c) { c != '`' }`. *Today:* `NK1171`.
*Others:* Rust and Kotlin `takeWhile/dropWhile`, Swift `prefix(while:)`,
`drop(while:)`, Python `itertools.takewhile`. *Fit:* the same shape as
`filter`, ends a `Seq` early, nothing new. *First recommendation:* add both;
lower priority than `any/all/find`, since one use here.

### Text: `rsplit`, `split_once`, `split_at`, and `Seq::last`

*Form:* `name.rsplit("::").next()`, `text.split_once('=')` -> `(String,
String)?`, `s.split_at(i)`. *Today:* `str::split`, `find`, `rfind` exist;
`rsplit` and `split_at` give `NK1171`, and `split(..).last()` too, since `Seq`
has no `last`. *Others:* Rust has all of them, Kotlin `substringAfterLast`,
`split`, Swift `split(separator:)`/`lastIndex(of:)`, Python `rsplit`,
`partition`/`rpartition`. *Fit:* text is a core type of the language, and
splitting at the last separator is basic in every one of the four. `split_at`
returns two views into one text, which is what ADR-283 D25 (one view
parameter) allows. Tuple results are refused as `(A, B)?` (D5 in
`docs/lowering-defects.md`, by design), so `split_once` would need its
absent case written as a struct or a decision on `(A, B)?` first. *First
recommendation:* `rsplit`, `split_at` and `Seq::last` (on a `Seq ... ends`)
now; `split_once` waits for the `(A, B)?` question, *not decided here*.

### `Seq::max`, `Seq::min`, `Seq::sum` (probe only)

Not in this material, met while probing: `v.iter().max()` is `NK1171` (`p35`);
`5.min(3)` works on numbers. Basic in all four languages. Recommendation:
decide them with `any/all`, as one round on `Seq` reductions.

## Already decided, so not gaps

| form | where | today |
| :--- | :--- | :--- |
| `loop` and `break value` | Part I (*There is no `loop` keyword. `break` hands back nothing*), ADR-276 D6 | `while true` + a `let mut` |
| postfix `?` on `T?` | ADR-023 D8 | `x ?? return null` (probe `p18`: parse error) |
| `while let` | not in Part I; the converter writes `let x = e ?? break` in a `while true`, which survived (`peeled`) | probe `q12` works |
| `if let` | not in Part I; `match` (rs2nika R24) or `x ?? default`; a `!= null` test does not narrow (see COMPILER) | probe `p13`: parse error |
| closure that changes a captured `let mut` | Part I 7 (lambdas) | probe `p16`: `n += x` in `for_each` gives 6, works |
| guard, or-pattern, range, literal in a pattern | Part I pattern table (`200..299` inclusive) | probes `p02`-`p06`, `q01`, `q05`, `q06` work |
| `{x:02x}` format spec | works (probe `p21`: `0a`) | the hand version wrote a hex table (STYLE) |

`if let`/`while let` are not decided anywhere (no ADR names them); the
`??`-forms cover every use in the material, so they come here as a remark and
not as a gap: *as it is today, until someone needs more*.

## COMPILER (3 in the material, 2 more from the probes)

| defect | n | Rust | by hand |
| :--- | ---: | :--- | :--- |
| `starts_with([chars])` passes the checker, rustc E0277 (D17) | 1 | `ty.starts_with(['i', 'a', ..])` | `render::an_or_a` |
| `match name { "i32" \| "i64" => .. }` on `ref String` (D19) | 1 | `matches!(name.as_str(), "i32" \| ..)` | `a_copied_scalar`, one arm per string |
| `mut e: ref E` reassigned in a loop and returned: rustc "missing lifetime" (new, probe `p25`) | 1 | `fn peeled(mut value: &Expr) -> &Expr { while let .. }` | recursion |
| `map.get(k)?.clone() ?? x` on a `Ty` or `Span` value lowers to a `&T` (E0277 `&Ty == Ty`, E0308); on an `i64` an index by `&i64` (D31) | 3 | `self.arm_types.get(&k)?` | `check_arm_leaves.nika`: `arm_type_of` returning `Ty?`, plain `?? return` for the `i64` |
| NK2302 on a method whose result holds `value`'s views and a `Ty` cloned out of `self`'s map | 1 | `fn arms_to_ask<'e>(&mut self, value: &'e Expr) -> Vec<(&'e Expr, Ty)>` | the clone moved into `arm_type_of` |
| two `self.s` arms of a Rust closure kept across a `self.s` write (E0502) | 1 | `leaves(then_branch)` called again after `arm_types.insert` | the booleans computed once |
| a guard over a `T?` scrutinee (`n if n > 3` on `i64?`) passes the checker, rustc E0308 (new, probe `p01`) | - | - | - |
| `if o != null { o + 1 }` passes the checker, rustc cannot add to `Option<i64>` (new, probe `q13`) | - | - | - |
| `-> (bool, String)?` and `-> (i64, Integer)?`: a nullable tuple as a result type does not parse ("Expected `{`") | 2 | `Option<(usize, Integer)>` in `arguments` | `check_arguments.nika`: an index (`-1` for none) and two helpers |
| a struct field `ref Expr?` filled from `ref Vec[Expr]` or `ref CheckWorld` params is NK2302 (whose data) | 1 | `Vec<(usize, Option<&Expr>, Ty)>` in `arguments` | `CheckArgument` keeps the arm's index, the view is read back where it is used |
| `if c { arm.tail } else { given[at] }` into `ref Expr?`: the plain arm is `Expr`, NK1103 | 1 | `given.get(at)` | helper `check_arguments_at` returning `ref Expr?` |
| `std::ptr::eq(whole, arm)` | 1 | `arguments` | the arm's index says it is an arm |

The last three belong in `docs/lowering-defects.md`; the last should be a
checker refusal (`NK1125`) if `!= null` is not meant to narrow.

## ADAPTER (4)

| Rust | why it stays |
| :--- | :--- |
| `claims_report` calls `check(parsed, own, library)` and reads `Ledger` | the checker and the ledger are Rust types; the Nikaia part takes `claims` |
| `several_errors` -> `Option<Vec<Ty>>` with `.filter(\|m\| !m.is_empty())` | Rust callers expect an `Option` of a slice |
| `payload_of` builds `parsed_view_of(parsed)` | `ParsedView` is the bridge to the Rust AST |
| `an_or_a -> &'static str` became `String` | no `'static` in Nikaia; the adapter converts |

## STYLE (9)

| Rust | by hand | needed? |
| :--- | :--- | :--- |
| `Some(Held::Precondition(f, 0))` arms | `if n == 0 .. else if n == 1` | no, nested literals work (`q05`) |
| `0x20..=0x7e =>` | `if byte >= 32 && byte <= 126` | no, range patterns work (`p04`) |
| `format!("{byte:02x}")` | a hex digit table | no (`p21`) |
| `rows.sort_by_key(..)` over a `BTreeMap` | dropped | yes, the map is ordered already |
| `.min(source.len())` | `if start > len { start = len }` | no (`p36`) |
| `.matches('\n').count()` | a `for` over `chars()` | no, `str::matches` exists |
| `count(\|h\| matches!(h, Held::Proved))` x5 | counters by `how.starts_with(..)` | no; a typed lambda works (`q17b`) |
| `from[1..]` byte slicing with `find` | the whole text as `Vec[scalar]` | partly: follows from no `split_at` |
| `Item::Enum { .. } if text == ty =>` | `if` inside the arm | no (`q06`) |

## Method

Originals: `git show <commit>^:crates/nikaia/src/check/mod.rs`, cut by brace
depth. Drafts: `crates/rs2nika/convert`, `nikaia build`, then
`target/nikaia/bin/rs2nika --profile check --file F.rs`. Probes: 43 small
programs run with `target/release/nikaia run`; names (`p01`..., `q01`...)
are the ones quoted above. A deviation is one place where the finished code
differs from the draft for one reason; a reason met in several functions is
counted once per function.
