1. **What cost the most:** about 15 turns went on finding my way through the checker and emitter. I had to work out where a `std` method's key is handled after the call is checked (`check_core_method_called`), how the checker and the emitter share facts (`check_facts.nika`), and where the emitter writes a method call (`emit_core_method_arm`). I also had to find out that structs the compiler generates derive no `Default`. That last point made D1's struct case an emitter change and not a simple pass-through. The tests-project run took the longest wall time, but it was a single run.

2. **What the brief could have handed me:**
   - That lowered structs have no Rust `Default`, so a struct `V` needs `or_insert_with(|| V { defaults })`.
   - Which existing code to use for the refusal. NK1102 was my reading of "a call whose type does not fit", and I had to dig it out of a 10 kB table line.
   - That message text uses `ty.text()`, and that `check_refusal` lives in `check_state.nika`.
   - That `.iter()` items are `view` types, and that a walk over numbers already gets `.copied()` (`copied_walks`). This decides when `cloned` lowers to nothing.
   - That the runtime `or_default` already exists in `map.rs`, so no twin was needed. The brief implied one was.

3. **Checks I ran:**
   - The old-binary probes of all shapes before building. These caught the open-number `sum` problem, which is outside my change.
   - Two build cycles, the second needed because of a help-text fix.
   - The tests project once, with `--bless`.
   - Clippy on 1.97 and 1.99.1, fmt, the `seq::` unit tests and `pre-push-check.sh`.
   - Wasted: the extra `cargo build` I ran for the fixpoint without first checking whether the second `lower-std` had changed anything. The first build was also partly wasted: the help-text slip (the f-string defect) only showed up when I ran the program.

4. **One change that would have saved the most:** a *Lowering facts* line in the brief giving where a std key is handled after the call and how a new fact reaches the emitter. For example: "`check_core_method_called` after `key` is set; add the fact in `check_facts.nika` (field and constructor); the emitter reads it in `emit_core_method_arm` before `emit_core_method_call`; structs derive only Debug and Clone (and Copy or PartialEq where the facts say so)". Ideally this would be a general *Adding a std method with custom lowering* section in builder-facts.md.
