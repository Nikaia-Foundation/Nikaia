# rs2nika score: why the kept share is low (#802 steps 1-2)

Corpus floors at `crates/rs2nika/convert/tests/corpus.in/manifest`; scorer
`rs2nika_score` in `src/main.nika`. Measured on the corpus of commit f38672b6.

## Exit status

`rs2nika --corpus DIR` exits 0 when every floor holds (the four lines, read
from `$?`, no pipe) and 1 when one is breached (checked with `function`'s floor
raised to 60 in a copy of the manifest). The `nikaia test` harness
(`project.rs`, `ended_well = status.success()`) fails the `corpus` test on a
non-zero exit, so a breach already fails CI; the harness has no "expected
failure" form, so there is no separate negative test.

## Is it the scorer?

No. The draft is the whole pre-move file, but the score is a multiset match of
final lines against draft lines, so unrelated draft lines can only raise it
(`return null`, `}` style lines match anywhere); restricting the draft to the
moved functions would lower it. A deeper normalisation (strip `check_<m>_`
prefixes, `self.`/`state.` alike, drop `let x: T`, `pub`, `sync`, `.clone()`)
was tried: attributes 62 -> 67, defaults 36 -> 42, function 6 -> 8, program
13 -> 27 of 440/187/49/137. Not worth the false matches; the scorer stays and
the floors stay. The low share is the converter's, and the moves'.

## What the sample shows (20 final lines per slice, nearest draft line)

1. The final is a redesign, not a translation. A `Checker` method becomes a free
   function threading `state, world, host`; the draft keeps `self.x(...)`.
2. Closure pipelines (`iter().find_map(fn ...)`, `visit_block(body, &mut |e| ..)`)
   become `for` loops with a different shape; the draft keeps the closure.
3. Rust text helpers become the Nikaia ones (`parsed.words.resolve(x).to_string()`
   -> `world.here.text(x)`), and `matches!`/slice patterns become `if` chains.
4. Findings: `self.findings.push(CheckFinding { code: ..})` -> `check_refusal(..)`.
5. New code with no Rust ancestor: new helper functions, `use` lines, notes.

## Lines the final has that the draft lacks (692 of 813, by kind; overlapping)

| Kind | Lines |
| :--- | ---: |
| control flow (`for`/`if`/`match`/`else`) | 176 |
| `state.`/`world.here` receiver text | 104 |
| call of a moved function with `state, world` | 66 |
| typed `let x: T =` (draft writes `let x =`) | 60 |
| `fn` signatures (names, `ref`, `sync`, host generic) | 60 |
| `match` arms `Pat => ..` | 53 |
| f-strings (draft keeps `format!` pieces) | 43 |

## Top 5 converter rules that would raise the score

1. Method -> free function with `state` threaded (`self.f(a)` -> `p_f(state, world, host, a)`,
   `self.x` -> `state.x`): about 170 lines (receiver text + calls).
2. Typed `let` for `String?`/empty collections/`mut` numbers, where the draft
   drops the annotation: about 60 lines.
3. Signature profile for a moved method (`ref`, host generic, prefix applied,
   no `sync` on a free function): about 60 lines. The manifest has no prefix
   column filled in, so the prefix rule is not scored yet.
4. `format!` -> `f"..."` including nested pieces and `.to_string()` joins: about 40 lines.
5. `iter().find_map/any/for_each` closures -> `for` with `return`/`continue`:
   most of the 176 control-flow lines, perhaps a third.

Next: fill the manifest's prefix column per slice, re-score after each rule,
raise a floor only with the rule that earned it.
