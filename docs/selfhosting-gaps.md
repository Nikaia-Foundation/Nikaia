# Self-hosting: what is still missing (2026-10-27, 0.0.1226)

A note for #125 and #579. It names what keeps Rust code from moving into
Nikaia **today**: forms the language lacks, gaps in the converter, and the
parts that stay Rust on purpose. Every language row below was probed today on
a small program with `target/fast/nikaia run`. Anything not listed was either
built since the first version of this note or never needed (see *Resolved
since*).

**Material** (the survival measurement, `crates/rs2nika/survival`, reads it):
three commits, thirteen functions: `5755b65` (`several_errors`,
`wholly_known`, `a_map_type`, `is_a_literal_other_than`, `a_copied_part`),
`3e26691` (`tail_of`, `articles`, `unpromised`), `07c5b14` (`payload_of`,
`claims_report`, `byte_literal`, `peeled`, `an_or_a`).

## Language: forms Nikaia does not have

Classed by the architect (2026-10-29, CLAUDE.md *A missing form stops the
work*): a *Written instead* that is not the designed form is debt, and a gap
row stops the move until its issue is decided and built.

| Rust form | Today (probe 2026-10-29) | Class | Written instead |
| :--- | :--- | :--- | :--- |
| `map.entry(k).or_insert(..)` | works on `HashMap`; NK1171 on `BTreeMap` | decided, ADR-collections-1 (#950 `ready`) | nothing until #950 is built; `HashMap::entry` as it is |
| `opt.as_mut()` on a `T?` | NK1125 | design, ADR-382: narrow, then write (`if o != null { o.n = 2 }`, lowered as `as_mut().unwrap()`) | the narrowed write; the old `?.clone()`, change, write back is debt (#953) |
| `map.get_mut(k)` | NK1171 | design, ADR-282 D7 (#816): `let mut c = m[k] ?? continue` is a mutable view; `m.get(k)` there is defect #952 | the mutable view; P5b's get-clone-insert is debt (#953) |
| `std::mem::take(&mut place)` | NK1181; NK1131 for a part of a `let mut` view | decided, ADR-views-1 (#949 `ready`) | nothing until #949 is built |
| `a.into_iter().zip(b)` | works (`Seq::zip`, ADR-293 D20) | not a gap | `zip`; P5b's index loop is debt (#953) |
| `s[..16]` on text | NK1214 | design, Part I 2.6 | `s.scalars.take(16).collect()` or `s.bytes[0..<16].text()` |
| `for mut x in v` | parser error | decided, ADR-views-2 (#951 `ready`) | nothing new until #951 is built; then `for mut x in v` replaces `iter_mut()` over a list or an array and the index loops |
| `m.values_mut()`, `m.iter_mut()` on a map; `let (a, mut b) = …` | parser error; after #951 `NK1307` | not decided yet: refused now, possible later (ADR-views-2 section 6) | a map's value changed by key, `m[k]`, over keys taken before the loop; `let mut (a, b) = …`. A move that needs more stops (`Result: blocked`) |
| `set.extend(xs)` on a `BTreeSet` | NK1171 | decided, ADR-collections-1 (#950 `ready`) | nothing new until #950 is built (158 loops already written, listed there) |
| `array.iter()` on `Array[T, N]` or a run | NK1171 | decided, ADR-collections-2 (#955 `ready`) | nothing until #955 is built |

Not gaps, by design: `Vec[ref i64]` is refused (NK1257, a view of a word is
the word); `grammar` is a reserved word; there is no `T??`: `a?.b` and a
read of a map of `T?` flatten (Part I 4.5, ADR-278 D11, ADR-293), and where
Rust's `Option<Option<T>>` keeps two kinds of absence apart (`reads_any` in
`comptime_run.rs`: keep, drop, drop under a name), the move names them as
an enum.

Not probed yet, met in the moves: `usize` arithmetic in indices
(`mem::replace` is #949), `'static`, `impl FnOnce(&mut Self)`, a struct of views taken
from a field of a `ref` struct (NK1106). (`ref T?` for Rust's `&Option<T>`
is `T?`: NK1261.) Each one becomes a row above, or a line under *Resolved since*,
when it is probed.

## Converter (rs2nika, #579, #802)

* `Option::as_mut()` on a field: no translation; the language's form is the
  narrowed write (ADR-382, above), which the converter could emit.
* `spec?.clone()` on a `String?` is written right only with `--facts`.

## Compiler defects that block a move

Filed as issues, never in this note: lowering defects #586, view structs
#627, a view result from two `ref` parameters needs a lifetime (#655 step 5,
ADR-353), NK2302 follow-ups #585, and #646, #647.

## What stays Rust, and why

| Part | Why | Issue |
| :--- | :--- | :--- |
| narrowing (`check/narrow.rs`) | the proofs are keyed by Rust spans and read by every check unit | #584 (C-7) |
| task work | blocked | #605 |
| `MultiLock`, `Constant` | Rust types the runtime owns | adapter |
| host methods of `CheckHost` | until C-5h moves the layer below the core | #584 |
| `CheckComparable` | a Rust trait over Rust types | adapter |
| `take_lend`, `insert_after_first_brace` in emit | no `String::pop` / `insert_str` in `std` | #583 |

The rest is not a gap but work in order: check C-5h, U-c1 to U-c11, C-7
(#584); emit U9, U10, E10, E11 (#583); the ADR-339 grammar, then
`parser/mod.rs` (#557).

**Where the Rust is** (`scripts/self_hosting.py`): 23 605 code lines in
`crates/nikaia/src` against 82 563 in Nikaia (77.8 %). Largest:
`project.rs` 4851, `parser/mod.rs` 4337, `emit/mod.rs` 2350,
`comptime_run.rs` 1697, `sysroot.rs` 1575, `modules.rs` 1392,
`contracts/mod.rs` 1326, `check/mod.rs` 1309, `main.rs` 1084.

## Resolved since

The first version of this note listed: slice patterns (`[a, b]`, `[a, .., z]`,
#618), narrowing of a struct or list `T?` (#621), a tuple pattern over an
option, vectors of views (`Vec[ref S]`), or-pattern arms with a guard,
`let .. else` (as `?? continue`), and the missing `std` entries of the
LANGUAGE section. All of these work today. Struct update was listed by
mistake: Nikaia writes `p with { a: 1 }` (Part I 4.2), and rs2nika R74
converts `S { a: 1, ..p }` to it.

## Method

Originals: `git show <commit>^:crates/nikaia/src/check/mod.rs`. Drafts:
`target/nikaia/bin/rs2nika --profile check --file F.rs`. A language gap is
listed only after a probe of a few lines fails on today's `develop`.
