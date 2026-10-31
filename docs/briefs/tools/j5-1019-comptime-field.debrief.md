1. **What cost the most:** the design search before any code. I traced how a `comptime` gets from the checker to the build-time run (checker, `compiled_build_time_value`, `compute_comptime`, Rust `sub_program`, `comptime_sub_program`) and tried five probe shapes. Typed lambdas, a concrete `Contact::fields` loop (NK1171) and a generic helper (NK1152) were all dead ends. Each blocker turned up only after reading a lot of code: the `sync.nika` defect for generic calls, and the decoder in `grammar_decode.nika` for step 3.
2. **What the brief could have handed me:**
   - **Files:** the decoder of build-time results is `grammar_decode.nika`, and step 3 needs it, but it was not on the allowed list. The sync check of a call is in `sync.nika`.
   - **Data flow:** `CheckFacts` is the emitter's `self.facts`, and Rust never builds it field by field, so adding a field to it is safe.
   - **The turn's state:** what the checker holds in a turn (`CheckUnrolling { on, field }`) and its matching emitter fields (`walk.specialising`, `walk.at_field`).
   - **Language syntax:** the explicit generic call syntax, and that it is not resolved by the sync check.
   - **How to run one test:** `nikaia test` takes no name filter, so I had to write a script for a scratch project.
3. **Checks:**
   - **Useful:** the probes on the old binary; three `lower-std` rounds with the old binary, each catching refusals (a lambda capturing its variables, `?.clone()` on an optional field) without a build; one build and the fixpoint; the 50 touched tests; fmt, clippy, `pre-push-check.sh`; the `nikaia-std` unit tests.
   - **Wasted:** the lambda probe, the concrete-loop probe and the `run`-on-a-test-file attempt. Also the early reading of `node_numbers.nika` for a rewriter I never wrote.
4. **One change:** before issuing a brief, the architect should check that every file each step must touch is on its allowed list. Here the step 3 decoder file was missing. The brief should also name the known gaps along the planned route (a sync call of an explicit generic) and say which design to use when the natural route is blocked.
