**Status: nothing is committed or pushed, and I wrote no report file.** Step 1 is partly built and uncommitted in the shared checkout:
- `scalar::from` exists: a ledger entry in `std.contracts`, the Rust helper `num::scalar_from`, an emitter mapping, and a fix so `scalar` no longer needs a `use` line.
- The prover now reads `std`'s preconditions: `scalar::from(0xD800)` is refused with NK1207.
- **Unproved calls are not refused yet (NK1300).** This fails when the precondition passes through a function the program wrote (for example a helper `g(n)` whose body is `scalar::from(n)`). The fix I had started, marking such claims as coming from `std`, is unfinished.
- Step 2 (json, `grammar_arrow`) is edited and checks clean. Step 3 (the refusal) is not started.

1. **Most turns:** finding out how a precondition in `std.contracts` is held at all. Nothing used `requires` there before. The prover read only the program's own ledger, never `std`'s. NK1300 only covers an `assert` written in the program. So "refused under `error`" needs new compiler work, which I found one probe at a time over three build cycles. It was also costly to learn that `scalar` is Rust's `char`, which rules out a Rust module `scalar` and forced an emitter mapping. The literal NK1241 is listed in the spec but no compiler code emits it.
2. **What the brief could have handed me:**
   - `requires` in `std.contracts` had never been used.
   - The prover's call path skips the library ledger.
   - NK1300 is tied to the program's own `assert`s.
   - `scalar` is a type alias for `char` in the prelude.
   - `scalar` is listed as a module because of keys like `scalar::to_string` (`check_walked.nika`).
   - NK1241 is never emitted today.
   - `rs2nika` already writes `(x as u8) as scalar`, since Rust's `as char` only takes a `u8`, so it needs no change.
3. **Checks run:** the probes under `run` and `check --asserts`, and `contracts::` plus `shipped_ledger::`, which passed (useful). Wasted: the first two probes of unproved calls, which used input-derived values that are "checked where the words arrive" by design. I misread that as a missing refusal.
4. **One change:** a short research brief first, or a *Locations* block naming `prover_walk.nika`'s foreign lookup, `check_claims.nika`, the alias in `lib.rs` and the absent NK1241. Better still, split step 1 into "std preconditions reach the prover and NK1300" as its own decided step. As written, step 1 hid a compiler feature behind "write it as a std function".
