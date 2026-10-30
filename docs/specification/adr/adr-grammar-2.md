# ADR-grammar-2: A Grammar Entry May Take a Type, and the Compiler Builds a Parser per Type; `members[S]` Fills a Struct by the Compiler's Rules, So a Typed Read Is a Grammar and Not a Library Walk

**Status:** Accepted
**Date:** October 30, 2026 (owner's decision on the architect's proposal, with the four recommendations of D11-D13 and D10)
**Answers:** how an official package offers a typed read, `json::read[T](text)`,
so that it is a parser specialised for `T` built from the package's grammar,
rather than a library function that walks `T::fields` and needs a way to build
a struct from field values that the language does not have (#604, #859)
**Supersedes / Related:** [ADR-344](adr-344.md) D3 (`read[T]` as a function
whose decoder is written while building: the call stays, the decoder becomes a
grammar), D6a (`parse`/`Value`: stays, as a second entry), §5 ("a typed read
needs a type, not a grammar": reversed); [ADR-330](adr-330.md) D1-D4 (a type as
a parameter, `T::kind`, `match` on a kind), D7 (a grammar names no type of the
program: its reason amended, the rule holds), D9 (`f[T]()` warned with
`NK1233`); [ADR-grammar-1](adr-grammar-1.md) D2 (a rule takes values; this adds
types); [ADR-specialization-1](adr-specialization-1.md) D1, D5, D7 (a copy per
build-time value, the build's budget, a result type from a `comptime`
parameter); [ADR-299](adr-299.md) D15-D18 (a row is the struct the block
names, columns matched by name); [ADR-331](adr-331.md) D5, D6 (field defaults,
attributes read as members); [ADR-304](adr-304.md) D6 (a generic original is
not emitted); [ADR-366](adr-366.md) (an entry reads `String` or `Bytes`);
[ADR-298](adr-298.md) (no macros)

## 1. Question

ADR-344 D3 makes `json::read[T](text)` a function of the `json` package that
unrolls `T::fields` while building. Reading the fields of a value is specified;
**building** a `T` from field values is not (`field.set` is `NK1180`), so the
function cannot be written. And a decoder written that way is a scanner of its
own: it gets none of a grammar's cuts, frames, streams, error form or views of
the input. **May a grammar entry take a type from its caller, so that the
compiler emits one parser per type and builds the value itself?**

Without an answer, `read[T]` waits for a construction form the language would
have only for this, and every format package (`json`, `yaml`'s serde layer,
`toml`, `csv`) writes its typed read as a hand-written walk beside its grammar.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Rust, serde + serde_json | `#[derive(Deserialize)]` writes a visitor per type; the format drives it; borrowed `&str` where possible | a macro system; the Deserializer/Visitor split keeps the format from seeing the known keys | from memory, not checked |
| simdjson On-Demand | a structural index, then a cursor the user's code walks; fields tried in declared order first | user-written cursor code; the index is a second pass | from memory, not checked |
| Go, easyjson | an external generator writes a decoder per struct with a hand-tuned lexer | generated files in the repository, a tool beside the compiler | from memory, not checked |
| Kotlin serialization | a compiler plugin writes a descriptor and a decoder per class; presence bits per field | descriptors looked up while running; an annotation on every class | from memory, not checked |
| Zig, `std.json` | `parseFromSlice(T, …)` switches on `@typeInfo(T)` at comptime; options per call; borrowed strings; a policy for duplicate fields | keys compared one by one; arms not checked until used | from memory, not checked |

Zig is closest: the type decides the parser at compile time, with no macro and
no run-time reflection. What does not transfer is the linear key compare and
the unchecked arms; what serde and easyjson add is a generated key dispatch.

## 3. Options, weighed

### Option A — a library function and a constructor from fields (ADR-344 D3 as written)

```nika
pub fn read[T: Struct](text: ref Bytes) -> T throws {
    let v = parse(text)                                   // a tree first
    return T::build(|f| from_value(f, v.member(f.name)))  // a construction form still to decide
}
```

### Option B — a grammar entry takes a type; the compiler emits a parser per type and builds the struct

```nika
pub grammar Json {
    entry rule read[T] -> T = v:Value[T] eof -> { v }
    rule Value[T] -> T = match T::kind {
        Struct(_) => Object[T]
        List(A)   => "[" xs:(Value[A] % ",") "]" -> { xs }
        Maybe(A)  => "null" -> { null } | Value[A]
        Text      => Str
    }
}
let c = Json::read[Contact](bytes)
```

### Option C — do nothing: only `json::parse` into a `Value`, converted by hand

```nika
let v = json::parse(text) catch { … }
let name = match v.member("name") { Str(s) => s, _ => … }
```

### Weighed

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | every format pays for a walk beside its grammar | the package author writes the decoder | a tree and a second pass at every read | a second way to build a struct, used only here | a construction form stays in the language |
| **B** | a format that has no typed read writes nothing | the compiler emits the parser and builds the value | one pass, no tree, known keys dispatched while building, strings as views | one mechanism for every format; SQL's mapping rule shared | type parameters on rules stay; `members` is one grammar element |
| **C** | every program pays | the developer writes the conversion | a tree and hand code at every read | consistent, but no typed read at all | programs full of hand conversion |

B wins on all four. The deciding weight is D3: the parser for `Contact` is the
only code that runs, and no value is built that the program does not keep.
Its claimed speed is not measured yet; the benchmark is a step of the build
(§5).

## 4. Decision

### D1 — A rule and an entry may take type parameters, given by the caller in brackets; the grammar is emitted once per list of type arguments.

```nika
entry rule read[T] -> T = v:Value[T] eof -> { v }
let c = Json::read[Contact](bytes)
```

One copy per rule and list of types, shared between calls; the generic
original is not emitted (ADR-304 D6), and the copies count against the build's
budget (ADR-specialization-1 D5). A bound may stand on a parameter
(`[S: Struct]`); a value where a type belongs is `NK1230`. The lowered rules
take no type parameter (`Value__Contact`), so the engine needs no generics. The
input is `String` or `Bytes` (ADR-366); a read of an `asset` in a `comptime`
works as any build-time call does (Part II 10.2).

### D2 — In a rule body, a `match` on a kind chooses the pattern, and only the arm the type takes is emitted.

Each arm is a pattern with its own actions, checked under the kind its pattern
names (ADR-330 D4). The match need not name every kind (D6). A `match` in a
pattern on a value not known while building is refused (`NK1310`).

### D3 — `members[S](key, value, sep; name:, null:, unknown:)` builds an `S: Struct` from named members; the compiler owns the matching and the building.

```nika
rule Object[S: Struct] -> S =
    "{" => m:members[S](Key, Value[_], ","; name: wire_name, null: Null, unknown: Skip) "}" -> { m }
```

`Value[_]` is applied with each field's type. One mapping rule, stated once in
Part II and shared with ADR-299 D16-D18:

* a missing member takes the field's default, else `null` in a `T?`, else it
  is an error;
* a member that matches `null:` goes into a `T?`, into any other field it
  takes the field's default, else it is an error (ADR-344 D4);
* a member given twice is an error;
* an unknown member is an error unless `unknown:` names a rule that skips it.

Refused while building, at the call: two fields with one wire name
(`NK1311`); a field that `name:` leaves unread, with no default and not a `T?`
(`NK1312`); a field type no arm covers (D6). While running, a failure is the grammar's
`ParseError`; the field names and indices on its stack are the path
(`$.contacts[3].email`, ADR-344 D3). No half-built struct exists in the
language, and `field.set` stays `NK1180`.

### D4 — A format's attributes are read in its `name:` function, a build-time function over the reflected field.

```nika
fn wire_name(f) -> String? {
    if f.attribute(Skip) != null { return null }
    return f.attribute(Name)?.value ?? f.name
}
```

The package writes it once; its grammar and its `write` use it (ADR-331 D6).
Each format reads its own attributes (`@json::Name`, `@yaml::Name`).

### D5 — Values follow the input, and the key dispatch is the compiler's.

A string with no escape is a view of the input (Part II 10.6), one with
escapes is decoded; a number goes straight into the field's type. How keys are
matched - by length and bytes, a trie, the declared order tried first - is the
compiler's, as a fixed map's lookup is (Part II 10.2). The skip rule builds
nothing, so an unknown member allocates nothing.

### D6 — A type the grammar has no arm for is refused at the call, naming the field's path and the rule; `refuse("…")` lets an arm say why.

```nika
Map(K, V) => match K::kind { Text => MapObj[K, V], _ => refuse("a JSON object's keys are text") }
```

`NK1250` is widened to this case. A function-typed field and a `Map[i64, V]`
are refused here.

### D7 — An entry marked `@stream`, whose body repeats an item on a fixed path, hands its items over one at a time.

It lowers to the engine's `#[stream]`. A JSON Lines file is read in parallel by
`par_fold` over a generic `@frame` rule, `init`, `step` and `merge` handed in as
rule values (ADR-grammar-1 D2). Reading from a pipe or a socket is not decided
here: an entry takes `String` or `Bytes` (`NK1282`).

### D8 — Writing stays a function per type.

`json::write[T]` walks the type with `match T::kind` and `T::fields`, builds
nothing it reads back, and shares `wire_name` with the grammar (ADR-344 D7).

### D9 — SQL keeps `dsl … -> T`; only its mapping rule is the one of D3.

A query's columns are known while building and its rows arrive typed from the
driver, so there is no text to parse while running. SQL keeps its strict
settings (no extra columns, no missing fields, ADR-299 D17) and checks them
while building; JSON uses its own and checks them while running.

### D10 — The `json` package offers `pub grammar Json` with the entries `read[T]` and `parse`, and `json::read` stays, forwarding to it.

`json::read[T](input; comptime schema: …)` is a `pub fn` of one line that runs
ADR-344 D6's schema check while building and calls `Json::read[T]`: the
spelling programs write does not change, and it is where `schema:` stands.
`parse` stays the entry for JSON of unknown shape (ADR-344 D6a). The
hand-written reader is retired.

### D11 — A type argument is written in brackets, and a call whose type argument fills no value parameter's type is not warned.

```nika
let c = json::read[Contact](bytes)        // no NK1233: nothing else says what T is
let n = parse[Config](text)               // still warning[NK1233] where the form is not needed
```

ADR-330 D9's `NK1233` is narrowed to calls where a value argument's type could
have said the type. `read(Contact, bytes)`, a type in a value's place, stays
`NK1230`: a type and a value are told apart by where they stand.

### D12 — A type with a text form of its own is refused as a field, until a record decides how a grammar meets it.

`json::Value` as a field, `time::Instant`, a `@json::Rest` field collecting the
skipped members: each needs an arm that names an exact type or is guarded by a
bound, which D2 does not have. Until that is decided such a field is `NK1250`
at the call, with its path.

### D13 — An enum variant without a payload is written as its name; a variant that carries one is refused for now.

```nika
enum Mode { Fast, Safe }
struct Job { mode: Mode }      // {"mode": "Fast"}
```

The name is the wire name of D4 (an attribute may rename it). A variant with a
payload is `NK1250` until its form (a tag member, an object of one member) is
decided.

**Rejected.** A library walk with a constructor from fields (Option A): a
second way to build a struct, and a scanner outside the grammar. `field.set`:
needs a half-built struct. A `Value` tree converted afterwards: two passes and
allocations. A trait any format implements (serde's split): the format cannot
specialise on the known keys. Descriptors read while running (Go, Kotlin): a
walk of the type at every read. Every kind match exhaustive: each format would
carry arms for `Fn` and `Opaque` that only refuse. SQL as a generic entry:
there is no text to parse while running.

## 5. Consequences

* ADR-344 D3, D6a and §5 say what holds now; its D4, D5 and D8 are JSON's
  settings of D3. ADR-330 D7's reason gains a sentence: a
  grammar still names no type of the program, it receives one from its caller,
  as a generic function in a package does. ADR-299 D16-D18 point to D3's rule.
* Part II gains text in 10.8 (type parameters on rules, `match` in a pattern,
  `members`, `refuse`, `@stream`, `Value[_]`), the mapping rule in 10.3 or a
  section of its own, and generic frames in 10.7; Part III 17.3 and Appendix
  C.3 follow. `NK1250` is widened, `NK1233` narrowed (D11), and `NK1310`-`NK1312` are new.
* The build waits on #859 steps 1-4; the package step also on a grammar
  exported by a package (#520, #485). The first slice is a grammar local to the
  program. Its speed is shown by a benchmark in `benches/json/` against
  `serde_json::from_slice` with derive, both release builds with the same
  settings, the same output byte for byte, counted with callgrind (#604).
* `yaml`'s serde layer (#928) and `toml` take the same mechanism.
* Accepted cost: type parameters on rules, a `match` in a pattern and one
  grammar element the compiler owns. A format's typed read is now as fast or
  as slow as the compiler's grammar lowering.

## 6. What this does not decide

* **How a grammar meets a type with a text form of its own** (D12): an arm
  naming an exact type, or one guarded by a bound. A decision round of its own
  ([#1020](https://github.com/Nikaia-Foundation/Nikaia/issues/1020)).
* **The JSON form of a variant that carries a payload** (D13).
* **A reader input** (a pipe, a socket) for an entry (D7).
