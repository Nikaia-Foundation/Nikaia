# ADR-catch-1: A `catch` Block That Begins With an Arm Is a `match` Over `error`; Every Other `catch` Block Means What It Means Today

**Status:** Accepted
**Date:** October 28, 2026
**Answers:** whether a handler that tells failures apart has to write `match error` inside its `catch`, and how a short form keeps every `catch` written today
**Supersedes / Related:** [ADR-023](adr-023.md) D1 (no list of error types in a signature), D4 (a `catch` matches with the language's patterns), [ADR-357](adr-357.md) D1-D5 (the set of types arriving at a `catch` is closed; a type named by its bare name; a complete `match` needs no `else`), [ADR-345](adr-345.md) D2 (a fatal error is taken only by name), [ADR-308](adr-308.md) (what the compiler says about a `catch`)

## 1. Question

```nika
let re = regex::dynamic(p) catch {
    match error {
        regex::Error => {
            eprintln(f"{error}")
            return
        }
        else => throw error
    }
}
```

A handler that tells failures apart writes two blocks, one inside the other,
and `match error` names the only thing a handler can tell apart. Can the arms
stand in the `catch` itself, and can that be added without changing a `catch`
already written?

## 2. How others do it

| | a handler over several kinds of failure |
|---|---|
| Java, Kotlin, C# | one `catch (T e) { … }` clause per type; what no clause names goes on |
| Swift | `do { … } catch pattern { … } catch { … }` |
| Scala | `try { … } catch { case e: IOException => … }`: the handler is a match |
| OCaml, F# | `try e with \| Not_found -> …`: the handler is a match |
| Rust | no `catch`; a `match` over the `Result` |

Taken from memory, not checked for this record.

## 3. Decisions

### D1 — A `catch` block whose first item is an arm is a `match` over `error`.

```nika
let re = regex::dynamic(p) catch {
    regex::Error => {
        eprintln(f"{error}")
        return
    }
    else => throw error
}
```

It means `catch { match error { … } }` with the same arms, and everything a
`match` over `error` is held to holds for it: the patterns of Part I 3.4, a
type named by its path or its bare name (ADR-357 D1), `else`, completeness and
`NK1151` where a type that can arrive is not covered (ADR-357 D1-D3), a fatal
error taken only by an arm that names it (ADR-345 D2). In an arm, `error` is
what it is in an arm of `match error`. The value of the `catch` is the value of
the arm taken, as the value of a `match` is.

**Why.** The only thing a handler can tell apart is the error, so `match
error` is a word the reader always knows; Scala and OCaml make the handler a
match for the same reason. A rule of its own would be a second set of rules
for one thing; the short form has none of its own.

### D2 — A `catch` block holds statements or arms, never both; a block of statements is unchanged.

No statement begins with `pattern =>`, so the first item decides which form a
block is, and no `catch` written today changes meaning: `catch { 8080 }`,
`catch { eprintln(f"{error}"); return }`, `catch { null }` and `catch { match
error { … } }` are blocks of statements as before. A handler that grows from
one answer to several moves its block into `else => { … }` and writes the arms
above it.

**Why.** Mixing - statements, then arms - would make an arm the end of a block
by position, which no other block has. **Rejected:** `catch match { … }`
(one more word for the same meaning; the arm already says it); clauses per type
in the manner of Java and Swift (a second way to write what `match` writes,
and incomplete by construction, against ADR-357's closed set); a new reserved
word.

## 4. What this does not decide

* Whether `error` in an arm that names a type is narrowed to that type, so
  that what only that type has is reached without a pattern that binds it.
  It would hold for `match error` alike and is a question of its own.
* A `catch` with arms over a joining block (`overlap`, ADR-308): the same rule
  is expected to hold, and the record that builds this checks it.

## 5. Consequences

* `catch { match error { … } }` and the short form mean the same; neither is
  deprecated, and nothing written today changes.
* The short form is built on ADR-357: it needs a type named by its bare name or
  its path to cover that member of the set.
