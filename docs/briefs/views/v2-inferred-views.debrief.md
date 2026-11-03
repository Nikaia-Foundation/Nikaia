1. **Most turns: working out that step 2 contradicts the specification.** About 8 turns went on probing b37's probe and its views-3 spellings, searching for an "existing escape error" that does not exist, then reading Part I 6.6, NK2303 and NK2304 to confirm it. Finding that step 1 was already built took about 4 turns of probes.
2. **What the brief could have handed me:**
   - **The error code.** The code for a view flowing where an own value is needed is NK1106 for a field and NK1102 for a `push`.
   - **No escape refusal exists.** NK2303 is not a refusal, and 6.6 anchors a struct handed back.
   - **#1152's probe.** It is already `tests/compiler/tests/ref_field_from_local.nika`.
   - **Ledgers of test programs.** They are not recorded, so "ledger lists the view" can only be checked by hand with `nikaia lower`.
3. **Checks:**
   - About 15 probes on the old binary; all were useful.
   - One `nikaia test --bless` in a scratch project, `fmt --check`, `pre-push`, and a by-hand ledger check with `nikaia lower`.
   - No build, clippy, `lower-std` or `it` run, since no code changed. Nothing was wasted except the grep for an escape-error message.
4. **One change: a pre-flight line in the brief**, written before it is issued: "probe each step's tests on the current binary; if they already pass, the step is tests only". The supervisor should also check every refusal the brief orders against Part I/III: an order that 6.6 contradicts needs a decision before the brief goes out, not after a builder run.
