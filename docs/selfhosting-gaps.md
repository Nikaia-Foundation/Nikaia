# Why so little of the rs2nika draft survives: tool gaps and language gaps

A note for #125 and #579. Of a draft by `crates/rs2nika/convert`, 22-35 % of
the lines reach the finished `.nika` file. This note takes the twelve
functions of three self-hosting commits, puts the Rust original, the draft
(`rs2nika --profile check --file`, today's converter) and the finished Nikaia
side by side, and sorts every hand correction into one of five kinds:

* **TOOL**: Nikaia has the form, the converter does not write it;
* **LANGUAGE**: Nikaia has no direct form, so the code was restructured by hand;
* **COMPILER**: the form is decided and the checker takes it, the lowering breaks
  it (a defect, not a gap; they are GitHub issues);
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

## TOOL (19 found, 17 since written by the converter or the compiler)

| form | n | Rust | the draft | by hand |
| :--- | ---: | :--- | :--- | :--- |
| `Option::as_mut()` on a field | 1 | `if let Some(outer) = self.s.flow.guarded.as_mut()` | `as_mut()` kept | clone, change, assign back (as `may_fail_here`) |
| `crate::ast::int_value` (`i128`) not in Nikaia | 1 | `int_value(*value, *negative)` | kept, listed as missing | compare the `u64` literal and its sign directly |
| LANGUAGE: `ref mut` parameter for a recursive walk's `seen` set | 1 | `seen: &mut BTreeSet<String>` | `NK1158` (*`ref mut` only for C*) | `mut seen: collections::BTreeSet[String]` |
| LANGUAGE: a method on `CheckState` handing back a struct of views of `self` and `world` | 1 | `Comparable { structs: &self.s.structs, own: self.world.own, .. }` | `NK2302`, and `self.structs` is not a `ref` field value (`NK1106`) | Rust builds `CheckComparable`; Nikaia takes it as `ref` in free functions |
| STYLE: `f"{name}{args:?}"` as a memo key | 1 | Debug of `Vec<Ty>` | not available | `name[` + `Ty::text` joined `]`, unique the same way |
| STYLE: `Vec::extend` with a `T?` | 1 | `refused.extend(found)` (Option is an iterator) | `NK1102` | `if found != null { push(found ?? continue) }` |
| LANGUAGE: a struct of views built from `self`'s fields inside a method | 1 | `Comparable { structs: &self.s.structs, .. }` in `declared` | `NK1106` (`state.structs` is not a `ref`) | the maps passed as `ref` parameters to a free function that builds it (`collect_keys_unordered`) |
| STYLE: `match op { Add => .., Sub => .. }` returning a tuple | 1 | `let (value, heading) = match op {..}` | kept as `if` chain over `op ==` | an `if`/`else if` chain assigning two `let mut` |

### For the converter, by frequency

Rows the converter now writes are gone from the table (items 1-7 of the first
round; the `filter(..).cloned().collect()` loop is R59; a lambda takes its
type from rust-analyzer's facts). Left for the converter:

1. `Option::as_mut()` on a field: still a TODO naming the hand pattern (clone,
   change, assign back); a rewrite needs the block's extent.
2. `crate::ast::int_value` (`i128`): a callee with no Nikaia twin; name one in
   `rs2nika.names` once it exists.

From the U-c1..U-c4 rows the converter now writes: a Rust name Nikaia reserves
(`grammar`, `throws`, ..) as `name_` (R69), and an assignment as a match arm's
value in a block (R62). Left for it: `spec.clone()` on a `String?` as
`spec?.clone()`, which needs the local's type from `--facts`.

## LANGUAGE (11): `std` entries, no syntax

| form | n | Rust | probe today | by hand |
| :--- | ---: | :--- | :--- | :--- |
| `Seq::all` / `Seq::any` | 5 | `parts.iter().all(wholly_known)` | built | a helper fn with a `for` and an early `return`; **removed** in `wholly_known`, `a_copied_part` |
| `Seq::find` / `Seq::find_map` | 3 | `items.iter().find_map(\|item\| match .. )` | built | nested `for` with `return` from a function of its own |
| `str::rsplit`, `Seq::last` | 1 | `name.rsplit("::").next().unwrap_or(name)` | built | a helper `type_last_segment`; its body is `rsplit("::").next()` now |
| `str::split_at` | 1 | `let (before, from) = rest.split_at(at)` | built | the text as `Vec[scalar]` and index loops |
| `Seq::take_while` | 1 | `from[1..].chars().take_while(\|c\| *c != '`').collect()` | built | a `while` over indices |

All five are `std` entries now (`std.contracts`: `Seq::any`, `all`, `find`,
`position`, `filter_map`, `find_map`, `take_while`, `last`, `max`; `str::rsplit`,
`split_at`); the sections below keep the reasons they were added for. The draft writes `.filter(!p).count() == 0`
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
parameter) allows. *Built:* `rsplit`, `split_at` and `Seq::last` (on a
`Seq ... ends`); `split_once` and `rsplit_once` since `(A, B)?` is a type
(ADR-364, #648): two views of the text as a `(ref String, ref String)?`.

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
| `starts_with([chars])` passes the checker, rustc E0277 | 1 | `ty.starts_with(['i', 'a', ..])` | `render::an_or_a`; **closed**: the checker refuses it (`NK1102`, one call per character) |
| `match name { "i32" \| "i64" => .. }` on `ref String` | 1 | `matches!(name.as_str(), "i32" \| ..)` | `a_copied_scalar`, one arm per string; **removed (0.0.965)**: one or-pattern arm |
| `mut e: ref E` reassigned in a loop and returned: rustc "missing lifetime" (new, probe `p25`) | 1 | `fn peeled(mut value: &Expr) -> &Expr { while let .. }` | recursion; **closed**: the loop runs with `v = ref inner[at]` (`NK1105` asks for the `ref`); `check_peeled` keeps the shorter recursion |
| `map.get(k)?.clone() ?? x` on a `Ty` or `Span` value lowers to a `&T` (E0277 `&Ty == Ty`, E0308); on an `i64` an index by `&i64` | 3 | `self.arm_types.get(&k)?` | `check_arm_leaves.nika`: `arm_type_of` returning `Ty?`, plain `?? return` for the `i64`; the clone is built, and the two later loops of this kind (`check_core_places_bound`, `check_core_flow_part`) are **removed (0.0.965)** |
| NK2302 on a method whose result holds `value`'s views and a `Ty` cloned out of `self`'s map | 1 | `fn arms_to_ask<'e>(&mut self, value: &'e Expr) -> Vec<(&'e Expr, Ty)>` | the clone moved into `arm_type_of` |
| two `self.s` arms of a Rust closure kept across a `self.s` write (E0502) | 1 | `leaves(then_branch)` called again after `arm_types.insert` | the booleans computed once |
| a guard over a `T?` scrutinee (`n if n > 3` on `i64?`) passes the checker, rustc E0308 | - | - | **closed**: the checker refuses it (`NK1102`) |
| `if o != null { o + 1 }` passes the checker, rustc cannot add to `Option<i64>` | - | - | **closed**: the test narrows, it runs |
| `-> (bool, String)?` and `-> (i64, Integer)?`: a nullable tuple as a result type does not parse ("Expected `{`") | 2 | `Option<(usize, Integer)>` in `arguments` | an index (`-1` for none) and a helper; **removed**: `(A, B)?` parses (ADR-364), `check_arguments` asks `keeps.iter().find` and `position` |
| a struct field `ref Expr?` filled from `ref Vec[Expr]` or `ref CheckWorld` params is NK2302 (whose data) | 1 | `Vec<(usize, Option<&Expr>, Ty)>` in `arguments` | `CheckArgument` keeps the arm's index, the view is read back where it is used; still: `NK2302` on a struct literal pushed into a local `Vec` that never leaves the function |
| `if c { arm.tail } else { given[at] }` into `ref Expr?`: the plain arm is `Expr`, NK1103 | 1 | `given.get(at)` | helper `check_arguments_at` returning `ref Expr?`; still: the checker takes it now, the lowering writes no `Some` on the plain arm (rustc E0308) |
| `std::ptr::eq(whole, arm)` | 1 | `arguments` | the arm's index says it is an arm |

What is still open here is filed as GitHub issues, not in this note.

## ADAPTER (4)

| Rust | why it stays |
| :--- | :--- |
| `claims_report` calls `check(parsed, own, library)` and reads `Ledger` | the checker and the ledger are Rust types; the Nikaia part takes `claims` |
| `several_errors` -> `Option<Vec<Ty>>` with `.filter(\|m\| !m.is_empty())` | Rust callers expect an `Option` of a slice |
| `payload_of` builds `parsed_view_of(parsed)` | `ParsedView` is the bridge to the Rust AST |
| `an_or_a -> &'static str` became `String` | no `'static` in Nikaia; the adapter converts |
| U-c1 `holes`: `hole_as_read` (a `Cow` over `parsed.hole_wraps`) and `literal_expressions_bound` | one host query `holes_read`, not the plan's `hole_wraps` + `wrap_hole`: a template's `<for>` names need the parsed `Parsed` too |

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
| U-c1 `match hole { V(n, _) => seen.insert(..), _ => {} }` | the insert in a block arm | no: the arm lowered without `;`, `Option` against `()` |
| U-c1 `fn compares(op: BinaryOp)` called with a bound `op` | the comparison inline; **removed (0.0.965)**: `check_core_ops_equality(op) \|\| check_core_ops_ordering(op)` | no: param lowered `&BinaryOp`, call `*op` |
| U-c1 `spec.clone()` on a `String?` from a pattern | `spec?.clone()` | yes (NK1125) |
| U-c1 `.or_else` chain of five literal kinds | `if x != null { return x }` per kind; **removed**: one `??` chain (`check_core_literal_by_use`) | no: `??` is lazy |
| U-c1 `let [condition] = args else {..}` | `if args.len() != 1` then `args[0]` | no slice patterns |
| U-c1 `message = Some(&option.value)` | the index of the option | no longer: `message = ref option.value` into a `ref T?` local runs |
| U-c2 `match (&evaluated, is_growable(want))` | a shape number from a helper over `ref BuildValue?`, then `if`s | no: no tuple pattern over an option |
| U-c2 `evaluated` read again after `evaluated ?? return` | `evaluated?.clone() ?? return` | yes (NK2105), at the cost of a clone |
| U-c2 `.get(..).and_then(|c| c.signature.as_ref())` | a helper over `ref FnContract`; **removed** (`check_core_comptime_result` read in place) | no: `c.signature ?? return` on a `get(..) ??` local runs |
| U-c2 `self.declared(ty, span)` and the `payload_of` / `{f:?}` closures | host methods `declared` (hosted), `payload_of`, `float_text` | yes: `Ty::from_ast` and `a_key_with_no_order` stay Rust |
| U-c2 `rust_constant_type(..).or_else(..).or_else(..)` | `if x != null { return x }` per step; **removed**: a `??` chain | no |
| U-c3 `door: MultiLock` across the crossing | `writing: bool`, the two spellings as helpers | yes: the Rust enum stays for `call`'s dispatch |
| U-c3 `methods.entry(k).or_insert_with(..).field = ..` | `remove(k) ?? empty()`, change, `insert` | no `entry` API on a map |
| U-c3 `kind: Option<(String, String)>` matched with a guard | a `bool` and two `String`s | no tuple pattern over an option |
| U-c3 `args.iter().map(\|(at, arg)\| match (arg, expected.get(at)) ..)` | a loop, a helper asking the pair, one function per arm | no tuple pattern over an option |
| U-c3 `args.split_last()` | `args[args.len() - 1]` after a length check | no slice patterns |
| U-c4 `MultiLock::named(name)` in `call` | the two spellings compared, `writing: bool` to `check_core_locks` | yes: `MultiLock` stays a Rust enum for the tests' ledger walk |
| U-c4 `name.strip_prefix(f"{ty}::")` | a helper over `ref String`; **removed (0.0.965)**: written inline | an owned `String` as a pattern lowers by value (E0277) |
| U-c4 `result: ref Ty?` handed a `Fn`'s field | the caller unwraps to `Ty` first | `ref T?` is `Option<&T>`, a bound field `&Option<T>` |
| U-c4 `rsplit_once` / `split_once` | `split("::")` collected, a loop | no longer: `std` entries since #648 |
| U-c4 `arguments`, `an_attribute_read`, `a_call_that_hands_its_number_back`, `a_standard_bound_not_met` | host methods (hosted) | yes: Rust until C-5h |
| U-c4 the `task::scope` half of `call` | host method `a_scope_that_pauses` | yes: #605's task work |
| U-c4 a trait method's `ref Vec[T]` | fixed in the compiler: a slice in the declaration too | no: was `&Vec<T>` against `&[T]` (test in `trait_mut_parameter.rs`) |
| U-c5 `CheckComparable` built from `state.structs` and the other maps | a free function taking each map as `ref` (`check_core_call_on_compares`) | yes: NK1106, a struct of references takes no field read of a `ref` struct |
| U-c5 `reached_method`'s `entry(current).or_insert_with` | `methods.remove(current) ?? empty`, changed, inserted back | yes: no `entry` API |
| U-c5 `Option<&Expr>` of the pausing lambda for the host | an index into `args`, `-1` for none | yes: an optional reference across the trait |
| U-c5 `witness: Option<usize>` | `i64`, `-1` for none | yes: no `if let` |
| U-c5 nested `if` inside `else if` of the push/insert binding | conditions joined with `&&` | yes: clippy `collapsible_if` on the lowered code |
| U-c5 the `task::Scope::spawn` half of `call_on` | host method `a_task_of_a_scope` | yes: #605's task work |
| U-c5 `a_lambda_that_pauses_at_an_entry` | host method, stays Rust | yes: not in the unit |
| U-c6 a local named `grammar` (`split_once` of the entry) | renamed `owner`; `find("::")` and two ranges | yes: `grammar` is a keyword |
| U-c6 `self.s.set_receiver.clone()` and the other `String?` fields kept for restoring | `?.clone()` | yes: NK1125, no method on a `T?` |
| U-c6 `matches!(args.as_slice(), [] \| [Ty::Unknown])` | `args.len()` and `is_unknown()` | yes: no slice patterns (#618) |
| U-c6 `std::mem::replace` of four flags | read into a `let`, then assigned | yes: no `mem::replace` |
| U-c6 the `(args, written)` rebinding to the list with the witness | the half after the door is its own function, called with `args` or the longer list | yes: no shadowing a `ref` parameter by a local slice |
| U-c6 `Constant::of(..).first_type()` | `integer_fits` for `i64` and `i32` | yes: `Constant` is Rust's |
| U-c7 `op ?? return` on a `ref BinaryOp?` | `let o: ref BinaryOp = …`, then `o.clone()` where a value is passed | no: a `ref` option unwraps to a `ref` |
| U-c7 `fn …() -> (Expr, Expr)?` for a range's ends | the range's half of `for` is its own function over `match iter` | no longer: `(A, B)?` is a type since #648 (ADR-364) |
| U-c7 `Local { .., ..self.local_free(..) }` | `CheckLocal::free` then field writes | yes: no struct update syntax |
| U-c7 `frame.first_mut()` | `frame[0].open_number = …` behind `frame.len() > 0` | yes: no `if let` |
| U-c9 `bound.get(p).cloned().unwrap_or(..)` | a loop over the map (`check_core_places_bound`); **removed (0.0.965)**: `bound.get(p)?.clone() ?? Ty::Unknown` | no: `bound.get(p)?.clone()`, which NK1185 names as the way, cloned the reference |
| U-c9 `match &self.s.flow.unrolling { Some(u) => .. }` | a helper over `ref CheckUnrolling?` | no: `state.flow.unrolling ?? return` moves out of `&mut` (E0507) on a field of a `mut` parameter of a type that does not copy; a `ref` parameter lowers `.as_ref()` and runs |
| U-c9 `field.parse::<usize>()` | digits checked, then `text::parse_i64` | yes: no `usize` |
| U-c9 `self.expect_at(..)` (a closure over `moved_to`) | `slots_waiting`, `expect_type`, `findings_moved` written out | yes: no `impl FnOnce(&mut Self)` |
| U-c9 `crate::contracts::locks::free_calls(self.parsed, body)` | a host method `free_calls` | yes: it needs the emitter's `literal_expressions` |
| U-c7 `Checker.got_past_a_jump`, `Checker.discarded` | moved into `CheckState` | no: Rust fields the moved code writes |
| U-c8 `self.with_proved(&names, \|this\| this.block(b))` | host methods `proved_here` and `proved_from_here`; the frame pushed in Nikaia | yes: the proofs (`Checker.narrows`) stay Rust in `narrow.rs` while #621 works on it |
| U-c8 `let outer = self.s.stamped_condition` and back | `?? -1`, then `null` or the value; **removed**: read and written back as the `i64?` | no |
| U-c8 `scope.last_mut()` with `frame[at].ty = ..` for a guard | the frame rebuilt; **removed**: `state.scope[last][at].ty = ..` | no |
| U-c8 `handed_over.as_mut().map(\|h\| mem::replace(..))` | `?.clone()`, the field set, the option written back | yes: no `as_mut` |
| U-c10 `whole(side, ty) -> Option<Option<String>>` | two functions, a `bool` and a name with `""` for none | no: a `T??` is not written |
| U-c10 `narrow::proved_names` + `with_proved` for `&&` / `\|\|` | `host.proved_here`, the frame pushed in Nikaia; `a_proof_taken_back` and `a_type_that_does_not_compare` are host methods | yes: narrowing stays in `narrow.rs` (#621); the parts beside stay Rust |
| U-c11 `match names.as_slice() { [ty, variant] if .. }` over a path | `names.len()` tests in order | no: no slice patterns |
| U-c11 `narrow::read_as_proved` and `reads_of_the_maybe` for a name | host methods; a read crosses only when its type is a `T?` and not `reading_only` | yes: narrowing stays in `narrow.rs` (#621) |
| U-c11 `build_time::an_escape_nothing_names(text)` with `char_of` | a stateless host method; `ESCAPES` written in the `.nika` | no: no `u32` to `scalar?` conversion was tried |
| U-c8 `arms.iter().skip(1).find_map(..)?` | a `while` over the arms | no: closures that return `?` |
| U-d0 `self.s.written_at[from..].iter().find(..)` | a `while` over the indices | no: no slice of a field in a condition was tried |
| U-d0 `enum Proved` and `Checker.narrows` keyed by `(usize, usize, i64)` | `CheckProved`, `CheckNarrowing` in `CheckState`, keys as `(i64, i64, i64)` | yes: the four fields travel with the baton |
| U-d0 `let Some(..) = .. else { continue }` | `?? continue` after a `!= null` test | no |
| U-d1 `stmts: &[&Stmt]` (the rest of a block after a `let`) | the block's `Vec[Spanned[Stmt]]` and a start index | no: no vector of references |
| U-d1 `Stmt::Break \| Stmt::Continue if !in_loop` | one arm each; **removed**: one or-pattern arm with the guard | no |
| U-d1 `let x: ref T = map.get(k) ?? return` | the annotation left out | yes: `NK1103` asks for the `ref`, `ref (map.get(k) ?? return)` runs |
| U-d1 `Checker.newly` (the build's `Newly`) | `CheckWorld.newly_throws`, `newly_keeps`; host `newly_throws` and `arguments` go | no |

## Method

Originals: `git show <commit>^:crates/nikaia/src/check/mod.rs`, cut by brace
depth. Drafts: `crates/rs2nika/convert`, `nikaia build`, then
`target/nikaia/bin/rs2nika --profile check --file F.rs`. Probes: 43 small
programs run with `target/release/nikaia run`; names (`p01`..., `q01`...)
are the ones quoted above. A deviation is one place where the finished code
differs from the draft for one reason; a reason met in several functions is
counted once per function.
