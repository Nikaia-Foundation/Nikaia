# A formal semantics of Nikaia? Decision basis

Owner's question 3 of [#796](https://github.com/Nikaia-Foundation/Nikaia/issues/796),
2026-10-24: before deciding, what a formal semantics is, where the work sits and
what it would look like. Companion to [`formal-verification.md`](formal-verification.md)
(item 10 of its table). Nothing concrete needs it now: no program, user or
measurement waits on it. It is the precondition of claim (a), a compiler proved
to preserve meaning, which is not aimed at.

## From zero

A program's **meaning** is what it does when it runs: the values it computes,
what it prints, where it stops. Today that meaning is written down in two
places: in the specification's prose (Parts I-III, about 6,700 lines), and in
what the compiler does, because a program is lowered to Rust and Rust decides
the rest. A **formal semantics** writes the meaning a third time, as rules in a
mathematical notation, so precise that a machine can check a proof about it.
It is to the prose what a grammar is to "a statement is roughly a line".

Take this program (probed on `target/fast/nikaia`, 0.0.1204):

```nika
enum Shape {
    Square(i64),
    Rect { w: i64, h: i64 },
}

fn area(s: Shape) -> i64 sync {
    return match s {
        Shape::Square(a) => a * a,
        Shape::Rect { w, h } => w * h,
    }
}

fn sum_to(n: i64) -> i64 sync {
    let mut s = 0
    let mut i = 1
    while i <= n {
        s += i
        i += 1
    }
    return s
}
```

`area(Shape::Rect { w: 3, h: 4 })` prints `12`, `sum_to(10)` prints `55`, and
`area(Shape::Square(4000000000))` stops the program: the text is Rust's
*attempt to multiply with overflow*, from the lowered `ex.rs`. `nikaia check
--bounds` keeps all four arithmetic checks (*a product of two unknowns*, #386;
*no fact bounds what it reads*, #432). `nikaia interpret` prints `{} {}`: the
interpreter (`tools/interpreter.nika`, 129 lines) walks `main` and evaluates
nothing (Stage 0). So no part of the toolchain today computes a program's
meaning except the compiled binary.

## The same three constructs, three ways

**Prose** (Part I, as written today):

* 2.2: *an arithmetic result that does not fit is an inconsistent program
  state, like an index past the end of a list or a division by zero, and the
  program stops.*
* 3.3: *a `while` loop repeats its body as long as its condition is true.*
* 3.4: *the `match` expression compares a value against a series of patterns*;
  `else` is *the arm taken when nothing above matched*.

**Formal, big-step** (one common notation; `σ` is the store, the values of the
names; `⇓` reads *evaluates to*; above the line what must hold, below it what
follows):

```
  ⟨e₁,σ⟩ ⇓ n₁    ⟨e₂,σ⟩ ⇓ n₂    n₁·n₂ ∈ [−2⁶³, 2⁶³−1]          ⟨e₁,σ⟩ ⇓ n₁    ⟨e₂,σ⟩ ⇓ n₂    n₁·n₂ ∉ [−2⁶³, 2⁶³−1]
 ───────────────────────────────────────────── MUL         ───────────────────────────────────────────── MUL-STOP
          ⟨e₁ * e₂, σ⟩ ⇓ n₁·n₂                                         ⟨e₁ * e₂, σ⟩ ⇓ stop(overflow)

      ⟨c,σ⟩ ⇓ false                         ⟨c,σ⟩ ⇓ true    ⟨b,σ⟩ ⇓ σ′    ⟨while c b, σ′⟩ ⇓ σ″
 ─────────────────────── WHILE-END        ──────────────────────────────────────────────── WHILE-TURN
  ⟨while c b, σ⟩ ⇓ σ                                   ⟨while c b, σ⟩ ⇓ σ″

  ⟨e,σ⟩ ⇓ v    armᵢ = pᵢ => eᵢ    ∀j<i: match(pⱼ,v) = no    match(pᵢ,v) = θ    ⟨eᵢ, σ ∪ θ⟩ ⇓ r
 ───────────────────────────────────────────────────────────────────────────────── MATCH
                          ⟨match e { arm₁ … armₖ }, σ⟩ ⇓ r
```

plus one rule per pattern shape for `match(p,v)` (seven plus eight in 3.4), a
rule for `stop` passing outward through every construct (*an `e₁` that stops
stops the product*), and the typing rules beside them: for example *a `match`
over an enum is complete when every variant is named*, the static side of
`NK1151`, which is what makes the `∀j<i … no` never run off the end.

**What writing them found at once** (checked in Parts I-II, 2026-10-24):

* **Operand order is not stated.** 8.1.1 fixes *statement* order; nothing says
  whether `f() * g()` runs `f` or `g` first. MUL above silently chose left to
  right. With a `print` in each function the two choices are two different
  programs; today Rust's left-to-right decides.
* **Which `stop`?** 2.2 says *the program stops*; A.2 of Part III gives the
  exit status; the text a user sees is Rust's panic message. A formal rule must
  say whether the message is part of the meaning.
* **Big-step says nothing about a loop that never ends**: WHILE-TURN has no
  derivation then, which is the same as "no meaning". Non-termination, `async`
  pauses and `overlap` need small-step rules (one step at a time) or a
  coinductive reading. That is a choice of style with real cost.
* **A `match` reads what it binds** (3.4, 6.5): whether an arm takes a view or
  moves the value changes what `s` is afterwards. `σ ∪ θ` above hides that; the
  real rule needs a store that knows owned, viewed and moved places.

## What it would be used for

1. **Finding gaps in the prose**, as above: the cheapest and surest gain.
2. **The statement claim (a) is proved against** (semantic preservation): no
   formal source semantics, no theorem.
3. **The meaning the prover reasons about.** Today *proved* means proved about
   the compiler's own reading of a program (ADR-270's certificates are checked,
   the facts they start from are not). A formal semantics is what a soundness
   proof of the prover, or of the checker (item 11), is stated over.
4. **A reference and test oracle**: an executable form runs a program, and the
   compiled binary must print the same (differential testing on the suite and
   on generated programs). This catches lowering defects without any proof.
5. **A second, independent implementation** for trusting trust (item 17).

## Where the work is

| Part of the language | Difficulty | Why |
| :--- | :--- | :--- |
| Integers, overflow, `wrapping_`/`checked_`, `if`, loops, `match`, structs, enums, calls | low | textbook; the rules above; floats need IEEE-754 as a library |
| `T?`, `??`, `throws`/`try`, `panic`, `break`/`continue`/`return` | low-medium | one outcome kind each, passed outward |
| Ownership, `ref`, views, the four doors (Ch. 6), the tether and the ledger | **high** | RustBelt and Stacked/Tree Borrows show the size: research-level; the typing side (what is refused) is larger than the running side |
| `async` by default, `overlap`, `spawn`, `user_parallelism` (Ch. 8) | **high** | needs small-step, interleavings and a memory model for shared data |
| `comptime`, build-time code (Part II) | medium | the same rules run at build time; the hard part is what the build may observe |
| Traits, generics, monomorphisation | medium | elaboration into a core first, then the core's rules |
| Text (bytes, scalars, graphemes), collections, `std` | medium, large | each `std` entry is a rule or a model; Unicode tables as data |
| Effects, input/output, `extern "C"`, `unsafe`, `assume` | outside | modelled as an oracle; `unsafe` is where the semantics stops promising |

Rough size, from comparable work and from memory, not measured: a core
(first two rows, no ownership, no concurrency) is a few hundred rules, some
thousands of lines in a proof assistant; the whole of Part I with ownership
and concurrency is comparable to the Wasm or CompCert C semantics, tens of
thousands of lines and person-years.

**Tools.** Proofs need a proof assistant: Lean 4, Rocq (Coq) or Isabelle (Wasm
has all three; CompCert Rocq; CakeML HOL4); the K framework gives executable
semantics with less proof power. A **Nikaia-hosted model** (the rules written as
a definitional interpreter in `tools/*.nika`) honours CLAUDE.md's rule that our
own tools are written in Nikaia and serves uses 1 and 4, but it cannot prove:
Nikaia's prover is linear arithmetic, not a logic for theorems about
programs. A proof in Lean is not a helper tool but it is code in another
language; that would be an exception to the rule, and it is the owner's.

## Cheaper forms, in order of cost

| Level | What | Serves | Cost |
| :--- | :--- | :--- | :--- |
| 0 | Prose as today | - | - |
| 1 | Paper rules for a core, beside the prose (as the Wasm specification does), not machine-checked | gaps (1) | small; drifts unless kept with the prose |
| 2 | **Executable semantics**: a reference interpreter in Nikaia, one function per rule, each citing its section; the compiled binary diffed against it on the suite and on generated programs | gaps (1), oracle (4), a second implementation (5) | medium; replaces today's Stage 0 interpreter, so it is a new build, not a check of an existing one |
| 3 | Mechanised core in a proof assistant; type soundness of the core | (2), (3) for the core | large; another language |
| 4 | CompCert-style preservation of the lowering | (2) in full | person-years (item 14) |

## Risks

* **Freezing too early.** The language moves every day (0.0.1204); each change
  to Part I would change rules too. Level 1 or 2 bounds that to the core.
* **Drift.** A third description that disagrees with the prose and the
  compiler is worse than two; it needs a check that ties it (level 2's diff).
* **The wrong target.** The meaning is partly Rust's today (the panic text,
  operand order); formalising Nikaia without settling those formalises the
  lowering's accidents.
* **Cost against self-hosting**: every builder on semantics is one not on #125.
* **Against the recommendation**: without level 3, *proved* (claim b) stays
  relative to the compiler's reading; a prover bug that agrees with itself is
  not caught.

## Recommendation (the owner decides)

Not levels 3-4 now. Level 1 for the core of Part I, written into the
specification as rules, is cheap and would already have found operand order;
level 2 is worth it once self-hosting has moved `check` and `emit` (#125 C-7
and E10/E11), as the oracle for a lowering that then no longer has a Rust twin
to be compared with. Not decided here: whether Part I takes formal rules at all
(STYLE.md would need a notation), and whether a proof assistant may ever enter
the repository.
