# ADR-yaml-1: `yaml` Is an Official Package in `packages/yaml/`: a Grammar Over `Bytes` Builds a Zero-Copy Tree With Spans, YAML 1.2 Core Schema, Limits On by Default; the serde_yaml Facade and `stream` Come Later

**Status:** Accepted
**Date:** October 30, 2026 (#928; the owner's decisions of 2026-10-28 and 2026-10-29 recorded there)
**Answers:** where the official package `yaml` lives, what its native API is,
what it is tested against, and in which order its later layers come (#928)
**Supersedes / Related:**
[ADR-344](adr-344.md) D1, D2 (an official package lives in `packages/<name>/` with its own version, taken in by path);
[ADR-338](adr-338.md) D2 (formats are official packages; `yaml` joins the list), §6 (editing a text in place: answered here for `yaml`);
[ADR-grammar-1](adr-grammar-1.md) (rule parameters, bounds, guards: YAML's indentation `n` and context `c`);
[ADR-296](adr-296.md) (a grammar is part of the language, lowered to the winnow-grammar engine);
[ADR-282](adr-282.md) D12, D13 (a field holding only views is a view position);
[ADR-016](adr-016.md) D5 (a grammar over a mapping checks UTF-8 as it reads)

## 1. Question

**What is the `yaml` package, as a program sees it, and what must stand before
its parts can be built?** #928 fixed the direction (a native API nobody fills
well, and a drop-in for the archived `serde_yaml` 0.9 for Rust users) and the
owner decided the shape (one package that is also the crate's core; features
`serde`, `tree`, `stream`; licences). Without this record a builder does not
know the input type, the tree's shape, what a limit is, nor which parts wait on
the toolchain.

## 2. How others do it

| Library | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| serde_yaml 0.9 (Rust) | `from_str::<T>`, `Value` that owns its strings; YAML 1.2 core schema (`yes` is a string); duplicate key an error; `<<` merged only by `apply_merge`; a recursion limit | every string allocated; no spans in `Value`; no comments kept; archived | its docs and source, 0.9.34 |
| saphyr / yaml-rust2 (Rust) | an event parser with markers, and a loader building an owned tree | owned tree; events carry a position, not a span | from memory, not checked |
| ruamel.yaml (Python) | round-trip mode keeps comments and order on edit | slow, fragile formatting rules | from memory, not checked |
| toml_edit (Rust, TOML) | a document model that keeps the text's formatting, edited in place | a second API beside `toml`'s | its docs |

None offers a tree that points into the input, carries spans and is edited in
place in one model.

## 3. Facts that bear on it

Probed with `target/fast/nikaia` on `develop` (2026-10-30):

* A grammar handed `ref Bytes` builds structs and enums whose `String` fields
  are views of the input, checked as UTF-8 while it reads, with `here` giving
  byte offsets, and a package exports them through a `pub fn` (works). `here`
  and `span` over a `String` are `NK1187` until `text::Pos` is built (#743).
  `fs::map` derefs to `Bytes` (Part III 17).
* A rule's parameters carry YAML's indentation and context
  (`tests/compiler/tests/grammar_yaml_block_structure_by_indentation.nika`).
* A plain function that builds a struct or an enum from views of a `ref`
  parameter's elements is refused (`NK1106`) or mis-lowered: #1000. A tuple
  result mixing `String` with another type refuses the views: #999.
* The engine (winnow-grammar 3b2dd3d) has `parse_<rule>_reader`, `#[stream]`
  and `#[stream(each)]`; the lowering does not write them yet.
* A package becomes a program or a C library only; `artifact = "rust-library"`
  and `[features]` are #978.

## 4. Decision

### D1 — `yaml` is `packages/yaml/`, a package with its own version; nothing here moves the language's version.

`packages/yaml/` holds `nikaia.toml` (`name = "yaml"`), `src/` and `tests/`,
like `packages/json/` (ADR-344 D1, D2). It is taken in by path. A change there
raises only the package's version. This record carries no Part I-III text, so
it raises nothing either. A compiler defect met on the way is fixed in
`crates/` as its own commit, which raises the toolchain's version as any such
fix does.

### D2 — The input is `ref Bytes`; positions are byte offsets.

The parse reads a `ref Bytes` (a mapped file, a buffer read from a pipe), so
spans exist today and a mapped file is read without a copy. A byte that is not
UTF-8 is an error at its offset. Line and column are computed from an offset
when an error or a tool asks. An entry taking `String` comes with `text::Pos`
(#743).

### D3 — One grammar builds the document tree, written after the specification's productions.

The grammar is a Nikaia `grammar` with rule parameters for the indentation `n`
and the context `c` (ADR-grammar-1), its rules named after YAML 1.2.2's
productions where one stands for one (`s-indent(n)` is `Indent(n)`), so a
reviewer can hold it against the specification. One grammar, no hand-written
scanner beside it.

### D4 — The tree points into the input and carries spans.

```nika
pub struct Node { pub start: u32, pub end: u32, pub anchor: String?, pub tag: String?, pub value: Value }
pub enum Value { Null, Bool(bool), Int(String), Float(String), Str(Scalar), Seq(Vec[Node]), Map(Vec[Entry]), Alias(String) }
pub struct Entry { pub key: Node, pub value: Node }
pub struct Scalar { pub raw: String, pub style: Style }   // Plain, Single, Double, Literal, Folded
```

Names may change in the build; the rules may not:

* Every `String` in the tree is a view of the input; nothing is copied while
  parsing. A scalar keeps its `raw` text as written; its value (escapes,
  folding, a block scalar's indentation) is computed on request and is a copy
  only then.
* A key is a node, not a string: YAML allows any node as a key.
* A mapping keeps its entries in the text's order. A document is a `Node`; a
  stream is a list of them.
* `Int` and `Float` keep their text, as `json` keeps a number (ADR-344);
  `as_i64`, `as_f64` convert on request.

### D5 — Plain scalars resolve by the YAML 1.2 core schema; a quoted scalar is a string.

`null`, `Null`, `NULL`, `~` and the empty node are null; `true`, `True`,
`TRUE`, `false`, `False`, `FALSE` are booleans; `[-+]?[0-9]+`, `0o[0-7]+`,
`0x[0-9a-fA-F]+` are integers; the core schema's float forms, `.inf` and `.nan`
in their three spellings, are floats; everything else is a string. `yes`,
`no`, `on`, `off` are strings. The core tags (`!!null`, `!!bool`, `!!int`,
`!!float`, `!!str`, `!!seq`, `!!map`) fix a node's kind; any other tag is kept
on the node.

### D6 — The parsed tree is the text as written; `load` resolves it.

The tree from the parse keeps `Alias` nodes, anchors and tags as written: it is
what spans, events and editing work on. `load` gives the tree a program reads:
each alias replaced by its anchored node (the structure copied, the strings
still views), a key given twice in one mapping an error naming both positions
(keys compared by kind and value), and `<<` an ordinary key: a merge happens
only on request, by a separate function, as in serde_yaml 0.9.

### D7 — Events are a walk of the parsed tree, in the yaml-test-suite's vocabulary, each with its span.

`events` turns the parsed tree into a list: stream, document (explicit or not),
mapping and sequence start and end (block or flow), scalar (style, raw text),
alias, each with anchor, tag and span. Printed one per line they are the
suite's `test.event` form (`+STR`, `+DOC ---`, `+MAP {}`, `=VAL :a`, `=ALI *x`),
which is what the tests compare. Events one at a time with fixed memory are
the `stream` stage (D10).

### D8 — Limits are on by default, a value the caller may pass, and an exceeded limit is an error.

`Limits` holds the nesting depth (default 128), the nodes an alias expansion
may produce (default ten times the document's own nodes plus 10 000: a billion
laughs stops, an ordinary anchor does not), and the input's size (default the
span type's range, 4 GiB minus one). The parse checks depth and size; `load`
checks aliases. An error names the limit, its value and the position. A caller
who passes a `Limits` raises or lowers them; none switches them off by
omission.

### D9 — The tests are the package's own, then the yaml-test-suite vendored byte for byte.

First, the package's own `tests/*.nika` with `.stdout`, run by
`nikaia test --project packages/yaml`. Then the yaml-test-suite's generated
`data` branch (one directory per case: `in.yaml`, `test.event`, `in.json`,
`error`), vendored unchanged in `packages/yaml/tests/suite/` with the suite's
MIT `License` and a file naming the source commit; a test program reads the
cases and compares, and the cases it passes are listed in a pass list outside
the vendored directory, which only grows. The `data` branch carries no licence
file of its own (it is generated from `main` at tag `v2022-01-17`, whose
`License` is the MIT text); the owner confirmed that `main`'s licence covers
it, with that `License` beside it (2026-10-30, #928).

**The vendored suite is never changed** (owner, 2026-10-28, again 2026-10-30): it
is copied byte for byte and stays so. The reason is a licence, not only honest
tests: the suite's Spec Example cases are copied from the YAML 1.2
specification, whose terms allow copying it only unmodified; MIT alone would
allow changes, the specification's terms do not. No case is edited, reformatted, renamed, re-encoded,
trimmed or deleted, not even one the parser fails or one that looks wrong; no
`cargo fmt`, `nikaia fmt`, line-ending or whitespace fix touches the directory.
What the package does not pass goes into the pass list beside it, never into the
suite. An update is a fresh byte-for-byte copy of a newer upstream commit, with
the commit file changed to name it.

### D10 — The serde_yaml facade, `stream` and editing are later stages, each waiting on a named step.

| Stage | Waits on |
| :--- | :--- |
| events and `load` over the tree (D6, D7) | #1000 (a struct built from a `ref` parameter's views) |
| the facade crate `nikaia-yaml`: serde_yaml 0.9's surface and behaviour, hand-written Rust over the generated crate, same version (owner, 2026-10-28, 2026-10-29) | #978 (`artifact = "rust-library"`, `[features]` with `serde` default, `tree`, `stream`) |
| `stream`: events from a file, pipe or socket in fixed memory | #978, and the lowering writing winnow-grammar's `parse_<rule>_reader`, `#[stream]` and `#[stream(each)]` |
| format-preserving editing (comments, order, indentation kept) | the tree carrying comments and every token's span; an ADR of its own for the edit model |
| writer matching serde_yaml 0.9's output | the facade's tests |
| typed `yaml::read[T]` on the Nikaia side | #859, as `json::read[T]` |

## 5. Consequences

* A program that reads YAML with `yaml` gets a tree that copies no text, with a
  span on every node. Nobody writes a YAML scanner by hand.
* A YAML file read as `String` must be handed over as `Bytes` until #743.
* A tree that points into its input lives no longer than the input; a program
  that keeps a value past it copies it with `.clone()` (ADR-282 D7).
* The pass list makes conformance a number that only grows.

## 6. What this does not decide

* The edit model's API (D10): its own record when the tree carries comments.
* Where the facade crate's hand-written files lie inside `packages/yaml/`:
  decided with #978's build, which fixes how a package becomes a crate.
* Python and JS layers: separate issues once the facade stands (#928 step 8).
