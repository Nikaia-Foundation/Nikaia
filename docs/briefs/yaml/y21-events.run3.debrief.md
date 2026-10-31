1. **Most cost:** turning the brief's probes into files. They sit in fenced blocks inside other briefs, so I wrote an awk extractor and read three briefs to find which output each probe wanted. After that, five small probe programs narrowed the NK2302 down to returning a view-holding value through an `if` or `match`. The rebuild took about 2 minutes, and it was needed.
2. **What I looked up:**
   - The shape of `Scalar` and that `text()` returns owned text. The brief says both "value through `text()`" and "views, not `.clone()`d". Those conflict for a `Value::Str`, and the brief should have settled it, for example with an owned `text` field beside the view.
   - D7's wording ("scalar (style, raw text)").
   - The fields of `Node`.
   - No converter, since this is no move.
3. **Checks:**
   - Useful: the pre-flight probes, fmt on a scratch copy, a one-file `check`, a `check` of HEAD's file (which showed NK1127 was a cascade), and the minimal shape probes.
   - Wasted: none, really. The package tests and the bless were rightly never reached.
4. **Something you should know:** I moved the local `wip/y21-events` ref with `git update-ref` (0433d662 → 1c1199d6) so the fix would not be lost. I did not delete the branch, but "leave a `wip/*` branch where it is" can fairly be read as "don't move it". If that was not wanted, the old commit is still 0433d662.
5. **One change:**
   - Put the pre-flight probes in the brief itself, inline, each with its wanted output, rather than in other briefs.
   - When the draft hits a view-checker refusal, the brief (or builder.md) should name the first diagnostic step: run the failing shape as `return x` and as `let r = x; return r`. That one pair would have reached this defect, and the y24–y26 ones, in two probes.
   - Better still, the architect's probe of the whole draft should run `check` on it before sending a builder, so all the view-checker defects show up in one batch rather than one run at a time.
