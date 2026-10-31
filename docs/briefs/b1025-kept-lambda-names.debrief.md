1. **Biggest cost:** I started the gate in the background with a 600 s limit, which is the foreground default, not the background maximum. It was killed partway through `tests/compiler`, so I had to re-run `tests/compiler` and clippy. That wasted about 10 minutes of machine time and two turns. After that, three quick `lower-std` rounds on slips I made myself:
   - `?? continue` used as a `for` header, which the parser refused;
   - `ref String` from iterating a local `Vec`, which needed `.clone()`;
   - `.cloned()` on a `Seq`, which doesn't exist (I used `.map(fn(p) { p.clone() })`).
2. **What I looked up:** the brief's locations were accurate. I still had to find:
   - `keeps_handed`'s signature, and that it doesn't take `holes_of`;
   - `seen_in`'s signature in `foreign.nika`;
   - `keep_viewed_name`, and the type of `walk.views`;
   - the `Seen` variants.

   One line saying "`keep_argument`/`keep_method_arguments` lack `holes_of`; `seen_in` is in `foreign.nika:67`" would have saved two reads.
3. **Checks:**
   - Useful: the new tests on the old binary (confirmed the failure), the build cycle (one build, lowered output stable), the `it` guard set plus `keeps::` (57 passed), and `tests/compiler` at both settings plus clippy (the second run).
   - Wasted: the killed first `tests/compiler` run. Also my glob `a_lambda_*.nika` ran three existing tests along with mine, a small waste.
   - Inaccurate: my report says `Disk freed: 0` although I deleted my copy of the binary. The real figure is a few hundred MB, not reported.
4. **Change that would have saved the most:** builder.md should say plainly: "a background run of `tests/compiler --both-settings` needs `timeout: 7200000`; the default 600000 kills it." It could also give the gate as one ready-made script with that timeout.
