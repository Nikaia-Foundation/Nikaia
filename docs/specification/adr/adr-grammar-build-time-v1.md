# ADR-grammar-build-time-v1: A Grammar Runs While the Program Is Built by Compiling the Parser It Generates, and Hands Back What It Found as a `meta::Type` the Compiler Writes a Type From

**Status:** Accepted
**Date:** November 20, 2026 (the consolidation, #904; the decisions themselves September 21 to November 19, 2026)
**Answers:** how a grammar runs while the program is built, and what crosses
from that run; how a grammar's holes bind; how a grammar that runs while the
program is built tells the compiler the type of something it found, when
nothing in the source writes that type
([#114](https://github.com/Nikaia-Foundation/Nikaia/issues/114),
[#178](https://github.com/Nikaia-Foundation/Nikaia/issues/178),
[#904](https://github.com/Nikaia-Foundation/Nikaia/issues/904))
**Supersedes / Related:** takes the decisions on a grammar run at build time
and what it hands back from the numbered grammar record now in the history
(its other decisions are in the other `grammar-*-v1` records); changes
[ADR-002](adr-002.md) D2 (a grammar run at build time is
compiled from the parser it generates; other `comptime` code is evaluated in
the compiler's process) and [ADR-database-v1](adr-database-v1.md) D2 (a block whose entry
returns a `meta::Statement` is a statement); [ADR-287](adr-287.md) D6 (the
build-time evaluator this runs inside, and what bounds it);
[ADR-310](adr-310.md) Q4 and D3 (the second compilation, named as a cost;
`asset("…")`, where the bytes come from, and what a build may read);
[ADR-311](adr-311.md) D1 (what a result has to be to cross);
[ADR-021](adr-021.md) (the build cache);
[ADR-grammar-syntax-v1](adr-grammar-syntax-v1.md) D5 and D6 (a grammar is
entered by an ordinary call, and every `entry rule` is an entry);
[ADR-330](adr-330.md) D3, D6, D7 (a `meta::Type`'s parts are types; it exists
only at build time; there is no `Named`); [ADR-304](adr-304.md) (reflection
reads a type's shape; no code as text); [ADR-297](adr-297.md) D5 (the
immediate/deferred axis); [ADR-133](adr-133.md) D3 (an options-only call
writes no `;`); [ADR-grammar-lowering-v1](adr-grammar-lowering-v1.md) D2 (the
lowering is syntactic); Part II 10.1, 10.2 and 10.5

## 1. Question

**How does a parse that runs while the program is built work, and how does
what it found reach the program, including a type nothing in the source
writes?**

Part II 10.2 promises **one grammar, two stages**: the same syntax and the
same meaning at build time and at run time. The run-time half is an ordinary
call. The build-time half, `comptime CONFIG = Json::value(asset("config.json"))`,
needs a way to run the parse, a rule for what its result may be, and a way for
a grammar such as a database driver's to say that a row has `name: String`
and `email: String?` when a type is not a value in this language and there
are no type arguments a grammar could write for the program. Without the
answers the build-time half would be a second implementation of the grammar
language, and every type a driver found would travel as text.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| `winnow-grammar` (the backend) | a code generator: its model crate parses and validates the grammar and hands it to a macro that writes a parser; no interpreter | none here; it is why D2 holds | its own summary, *intended to be used by procedural macros that generate parsers* |
| Zig | `@Type(info)` turns a description into a type; types are build-time values throughout | the whole language carries types as values, and an `anytype` failure lands in the body | from memory, not checked |
| F# type providers | a library run by the compiler supplies types from a schema | a compiler interface of its own | from memory, not checked |
| Rust `sqlx::query!` | a macro reads the schema and writes a struct as tokens | code generated behind the reader's back | from memory, not checked |
| TypeScript with Prisma | a generator writes type files before compiling | a second build step | from memory, not checked |

What transfers: running the generated parser rather than a second
implementation (D2, D3), and Zig's *description as data* (D7). What does not:
types as values everywhere, and generated source the reader did not write
(D10).

## 3. Options, weighed

| Option | Why it lost | See |
| :--- | :--- | :--- |
| Interpreting the grammar tree in the compiler | a second implementation of the same semantics, whose disagreements land in the corners (implicit whitespace, repetition bounds, the commit point, frames, interning, spans) | D2 |
| Building a parser to learn that its answer cannot cross | the declaration already says it; the build is wasted | D5 |
| Re-wording the parser's diagnostic | its words and positions mean something only against the bytes parsed | D6 |
| One binding mechanism for holes | scope binding is right for assembly and destroys the reuse of a prepared statement | D11 |
| Scalars only (`Text`, `I64`, … and nullable) | enough for SQL, a regex's named groups and a CSV header; not for JSON, Protobuf or GraphQL, which nest, repeat and name | D7 |
| The type as text (`"String?"`) | a misspelling surfaces when the grammar runs for a user, not when the driver is compiled | D7 |
| Any `comptime` may compute a type (`comptime type Config = …`) | a construct of its own, with its own questions of naming and identity; every format named above is read by a grammar | §6 |
| `meta::column(name, type)` and `meta::parameter(name, type)`, one call per column or parameter while the grammar runs | a driver knows a statement's shape only once it has read all of it, and a side effect of an alternative tried and given up would already have declared its columns | D8 |
| A call per table; a namespace written into the name (`"a.id"`) | the compiler would take the name apart as text | D8 |

## 4. Decision

### D1 — One spelling at build time and at run time; *when* is said by `comptime`, *where the bytes come from* by `asset("…")`.

```nika
comptime CONFIG: Json::Value = Json::value(asset("config.json"))
let data = Json::value(user_input)
let query = dsl mysql { … } eod
```

A parse has one spelling at either stage, and its position never decides its
meaning: the same call reads its argument's text at run time, and at build
time only because `comptime` asks for it and `asset` names the file. A `dsl`
block is an expression. The call says *what is done with the bytes*, the third
word beside the two [ADR-310](adr-310.md) D3 named; what a build may read is
[ADR-310](adr-310.md).

### D2 — A grammar is **not** interpreted while the program is built, and the reason is not the size of the work.

The compiler holds the grammar tree, and walking it against the input is the
short path; it is refused. `winnow-grammar` is a code generator and has no
interpreter to borrow, so interpreting here would be a second implementation
of the same semantics, and the two would have to agree exactly. Part II 10.2's
promise would stop being a property and become a hope, and the disagreements
would show as *this file parsed while the program was built and fails while it
runs*, for the same file and the same grammar. Other `comptime` code is
evaluated in the compiler's process ([ADR-287](adr-287.md) D6); a grammar is
not.

### D3 — The **generated** parser is compiled and run.

The compiler writes a small Cargo project holding the grammar's own lowering,
the types its rules produce and a driver, builds it, and runs it over the
bytes. There is one implementation of the grammar language, and the agreement
between the stages is a tautology rather than a claim.

It needs no new security model: a grammar's action blocks are Nikaia, and
[ADR-287](adr-287.md) says what a build-time body may do; the bytes come from
`asset("…")`, and [ADR-310](adr-310.md) bounds which. A grammar inside a
grammar is refused: the inner run would have this compiler start a second
compiler inside the first, a build that does not end.

### D4 — The cost is a second compilation, and the cache is what makes it bearable.

[ADR-310](adr-310.md) Q4 named this cost. The sub-project is keyed on the
program text that went into it (the grammar's lowering, the types, the dump),
so it is written and built when the grammar changes and reused otherwise
([ADR-021](adr-021.md)). The compiler already emits Rust and drives Cargo, so
the machinery is not new.

### D5 — The result has to **cross**, and a result that cannot is refused by name.

What a `comptime` hands the program is a value the build carries and the
language below can write: [ADR-311](adr-311.md) D1's set, an integer, a finite
float, a `bool`, text, a list, a pair, a `struct` and an `enum` variant, each
in its view form. A rule whose declared result is outside it has nothing to
arrive as, and is refused **before a parser is compiled**, from the rule's
declaration rather than from what it produced: `NK1178` names the type and
says why it cannot cross. What reaches the generated file is the answer, and
no parser runs while the program does.

### D6 — Invalid input fails the build, in the parser's own words.

Part II 10.2 A says it and this keeps it literally: the parser's diagnostic is
**relayed whole**, not re-worded. It is in the grammar's own vocabulary and
counts its line and column against the bytes that were parsed, the one place
they mean anything.

### D7 — `std::meta::Type` describes a type as data.

A `meta::Type` is a type's kind, carrying the types it is made of, as
[ADR-330](adr-330.md) D3 lists them: the scalars (`I64`, `I32`, `U8`, `U64`,
`U32`, `F64`, `Bool`), `Text`, `Bytes`, `Maybe(A)` for `A?`, `List(A)` for
`Vec[A]`, and the other kinds that record names. A grammar builds a type the
compiler writes with `Record(fields)`, each field a name and a type, and names
only the types its own package can name: there is no `Named`, and a grammar
names no type of the program ([ADR-330](adr-330.md) D7). A `meta::Type` exists
only while the program is built; one that would reach the running program is
`NK1231` ([ADR-330](adr-330.md) D6).

The type checker checks a description when the grammar is compiled, so a
driver's mistake is the driver's, at its line, and a shape that is not a type
cannot be written.

### D8 — A grammar hands its description back as the value of its entry; there is no `meta::column` and no `meta::parameter`.

A grammar that describes a statement returns a `meta::Statement` from the
entry the block runs:

```nika
pub struct Statement { row: Type, parameters: Type }

main entry rule statement(; schema: Schema) -> meta::Statement = q:Select -> {
    meta::Statement { row: q.row_type(schema), parameters: q.parameter_type(schema) }
}
```

`row` is the type of one result row and `parameters` the type of the values
the statement takes, each a `Record`. A namespace is a field whose type is a
`Record`: a database driver gives each alias of a query one, holding its
columns ([ADR-database-v1](adr-database-v1.md) D22). A block whose entry returns a
`meta::Statement` is a statement ([ADR-database-v1](adr-database-v1.md) D2). The value crosses
as data and never reaches the running program: what the program holds is the
statement the compiler builds from it.

A driver knows a statement's shape only once it has read all of it (the
`FROM` that says what `a` is comes after the `SELECT` list, and `SELECT *`
needs the schema), so it builds its own model first, and a call per column
would only translate that model into side effects. A side effect of an action
is also wrong where a rule's result is discarded: an alternative tried and
given up would already have declared its columns. A value is what the rule
that matched hands back, and nothing else.

### D9 — The compiler writes the type, and only it.

From the `row` and `parameters` of the `meta::Statement` the block's entry
returns, the compiler writes the block's row type and its parameter type
(D12). `Text` is `String`, `Maybe(T)` is `T?`, `List(T)` is `Vec[T]`, and a
`Record` is a struct with those fields.

### D10 — A written type says where it came from.

A message about a type the compiler wrote names the block and the grammar
that declared its members (*`email` is a column of the query at line 12*), and
`--comptime` prints the type each block was given. The type is data the reader
can ask for, never source the reader did not write ([ADR-304](adr-304.md)).

### D11 — A hole binds in one of two ways, and the grammar author says which.

`meta::capture(id)` is **immediate**, resolved from the surrounding scope at
compile time (assembly: `$v = in(reg) val` means *this* `val`). A deferred
hole is a named runtime parameter (a prepared statement, which baking values
in would destroy), one of the `parameters` of the `meta::Statement` its
grammar returns (D8). It is the same immediate/deferred axis as lambda capture
and teardown ([ADR-297](adr-297.md) D5), and Part II 10.5 states it as a
principle.

### D12 — A DSL with deferred parameters generates a shadow type.

The `:name` holes of a body become a type the call site is checked against: a
missing name is `NK1112`, an unknown one `NK1113`, on the `.nika` line. The
parameters are configuration, passed as named arguments after the subject,
`query.execute(db; target_age: min_age)`, or `stm.execute(target_id: 501)`
with no `;` when there is no subject ([ADR-133](adr-133.md) D3), and a driver
receives them with the typed spread `...args: Self::dsl`, which keeps its `;`
in the signature. Following
[ADR-grammar-lowering-v1](adr-grammar-lowering-v1.md) D2, the field types come
from the call site (the struct is generic and monomorphised where it is
built), there is one struct per parameter list rather than per DSL string,
`Self::dsl` also names the result, and a hole is found by scanning, so
`SELECT ':id'` is read as one until the `parameters` its grammar returns say
otherwise (D8). The driver's own half, `args.values()`, needs a value type for
the parameters and is not decided here.

## 5. Consequences

* **Part II 10.2 A is a program.** `comptime` says *when*, `asset("…")` says
  *where the bytes come from*, and the call says *what is done with them*.
* **The parse leaves nothing behind**: the generated file holds the answer.
* SQL needs no exception: what a block's grammar finds becomes a type through
  one channel, which a regex, a CSV header or a JSON schema use as well.
* `meta::Type` holds itself (`Maybe(Type)`), so it rests on a type that holds
  itself through a box ([#97](https://github.com/Nikaia-Foundation/Nikaia/issues/97)).
* Accepted costs: a Cargo project per grammar, built once and reused; a
  generated encoder and a decoder that reads it, held together by tests that
  run programs; `NK1112`/`NK1113` only where the scan can answer (D12); the
  shadow type is the least proven decision of the protocol.

## 6. What this does not decide

* **Which results cross**: D5 refuses whatever a build-time value cannot be,
  and that set is [ADR-311](adr-311.md)'s.
* **The driver's half of D12**: `args.values()`, a value type for the
  parameters, and `SqlAst[Signature = {…}]` as a spelling.
* **Whether two blocks with the same columns have one type**, or each block
  its own.
* **A type computed by any `comptime`**, outside a grammar.
* **Whether `meta::capture`** (D11), the immediate binding of a hole, also
  becomes part of a description a grammar returns (D8).
* **How a grammar receives a block's build-time arguments** (`schema: APP`),
  [#115](https://github.com/Nikaia-Foundation/Nikaia/issues/115)'s build.
