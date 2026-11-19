# ADR-text-reuse-1: A Changed View Takes Its Source's Buffer, `replace` Works in Place, and `.to_string()` on Text Is Refused

**Status:** Accepted
**Date:** November 14, 2026
**Answers:** whether a cut of text that the program changes, and a `replace`
whose source is not read again, must allocate new text, and whether
`.to_string()` on text keeps a job (issue
[#1589](https://github.com/Nikaia-Foundation/Nikaia/issues/1589))
**Supersedes / Related:**
[ADR-282](adr-282.md) D8 (`.to_string()` of text is the text itself; changed
here for a receiver written as text), D7 (`.clone()` is the one word for a
copy), D30 (`NK1301` on a literal; widened here);
[ADR-views-3](adr-views-3.md) D1 (a give is not written), D2 / D9 (use after a
give), D6 / D10 (the ledger narrates);
[ADR-005](adr-005.md) §3 (a copy is the program's to write);
[ADR-258](adr-258.md) (the four weights);
issue [#577](https://github.com/Nikaia-Foundation/Nikaia/issues/577) (the same
idea for tree nodes)

## 1. Question

A cut (`s[a..]`, `trim`, `split`) is a view, and a view is a `String`
(Part I 2.3). When the program then changes the cut, or replaces text in a
`String` it never reads again, must the change work on new text, or may it
work in the source's buffer; and is anything written in the source for it?

Without an answer a changed cut either allocates a copy nobody asked for, or
(today) lowers to a changed `&str` that the language below refuses.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Rust | `String::drain`, `truncate`, `replace_range` reuse the buffer, but the programmer writes them; `str::replace` always allocates | A second spelling for each cut | std docs, from memory |
| Lean 4, Koka | "Functional but in place": a unique value is changed in place, decided by a reference count at run time | One count test per change | Lean 4 / Koka papers, from memory |
| Swift | Copy on write: a unique buffer is changed in place, checked at run time | One uniqueness test per change | from memory, not checked |

What does not transfer: the run-time check. Ownership is static here
(ADR-005, ADR-094, ADR-views-3), so the compiler knows who owns a text and
whether it is read later, with nothing to test while the program runs.

## 3. Options, weighed

### Option A — No word: a changed view takes its source's buffer

```nika
let m = line.find(";") ?? return line
let mut rest = line[m + 1..]
rest += "!"
return rest
```

### Option B — An explicit word, e.g. `.own()`

```nika
let mut rest = line[m + 1..].own()
```

### Weighed

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | Only a program that later reads the source pays, with the existing use-after-give error and its `.clone()` help | Yes: the compiler finds the give | A give allocates nothing (`truncate` O(1), `drain` one move of the kept part) | A give is never written (ADR-views-3 D1); a changed literal already becomes text of its own unwritten (Part I 2.3) | A read added later turns the change into an error that names both places |
| **B** | Every changed cut writes the word | The developer marks what the compiler knows | Same cost | A second word beside `.clone()` for "text of its own", which Part I 2.3 refused on purpose (`NK1189`) | The word marks exactly the places where nothing is paid |

`.own()` on a live source would either copy (then it is `.clone()`) or refuse
(then it says nothing the compiler does not know). B lost on D4.

## 4. Decision

**The rule all of it follows: a copy is written, a give is not.**

### D1 — `.to_string()` on text is refused.

On a receiver whose written type is text (`String`, or a view of it) the call
is refused with `NK1301`, as on a literal (ADR-282 D30) and as `.to_owned()`
is (`NK1189`): a view is a `String` already, and the compiler decides whether
text is a view or text of its own. The help drops the call, or writes
`.clone()` where a copy is meant; the refusal carries that edit, so
`nikaia fix` makes it. On a type parameter (`fn show[T: Show](x: T) -> String
{ return x.to_string() }`) it stays allowed, also where `T` is `String` at a
call: there it is the text form of a value, and for text the text itself.
`.to_string()` stays the text form of every other value.

### D2 — Changing a view gives its source to it.

A view that is changed (assigned to, `+=`, a `mut` method, handed to a `mut`
or `keep` parameter, kept in an owned slot) is text of its own. It takes the
source's buffer when the source can be given: it is owned here (a local, a
by-value binding, a `keep` parameter) and no other view of it lives past the
change. `s[a..]` lowers to `drain(..a)`, `s[..<b]` to `truncate(b)`, a cut of
both to both, `trim` to the same at the trimmed ends. Nothing is written in the
source code. Where the kept part escapes (returned, kept, pushed) and is less
than half the buffer's capacity, the lowering shrinks it; a local is not
shrunk.

### D3 — After the give, the source is gone.

A later read of the source, or a change of a second view of it, is the
use-after-give error of ADR-views-3 D2 / D9, with its `.clone()` help; the help
names the change that gave the source. No new code. A view of a **lent**
parameter cannot be given: changing it needs `.clone()`, or the parameter says
`keep`, and the error names both.

### D4 — `replace` on a source not read afterwards works in its buffer.

A choice of how the call is lowered, not a rule: `replace` reads its receiver
and hands back new text, and that stays. Where the source is dead
(`x = x.replace(a, b)`, `let y = x.replace(…)` with `x` not read again, every
link after the first of a chain), the result is made in the source's buffer:
one forward pass then `truncate` where the replacement is not longer than the
needle, else a `reserve` of the difference and a fill from the back. Where the
source is read later, or is a view, the result gets a buffer of its own, as
before. No second method: one way to say it.

### D5 — The ledger narrates each give and each reuse.

"`rest` takes the buffer of `line`", "`replace` works in the buffer of `x`"
(ADR-views-3 D6 / D10); the language server shows each as a hint.

## 5. Consequences

`.to_string()` leaves code on text; a program that wrote it drops it, which
changes nothing it prints. A changed cut and a dead-source `replace` allocate
nothing; what is copied is still written as `.clone()`. A buffer kept for a
small escaping part is shrunk, so the cost accepted is a `realloc` there. The
work is issue #1589.

## 6. What this does not decide

The names `before` / `after` / `between` for cuts are ADR-374's; until they
are built the rules apply to `s[a..]` / `s[..<b]`.
