1. **What cost the most:** designing `group.rs` (the `Receiver` trait, slots, the deferred `ChildHandle`, and `Deferred` as a separate type because `TaskHandle::start` is a constructor) and reading `task.rs`'s four scope kinds to learn what to keep. The context ran out once along the way. After it, the `tests/compiler --both-settings` run took the most time (642 programs, in the background). One clippy lint (`manual_async_fn`) cost an extra round.
2. **What I looked up that the brief could have handed me:**
   - The secondaries of D21 are diagnostic only (Part I 7.1), and a program test must exit 0. So "two failures both kept" cannot be seen in a passing program. The brief's test list implies it can.
   - Step 2 has no Nikaia surface at all before #988. The brief could have said "partial is expected" up front.
   - The working program shape: tasks cannot assign outer `mut` (NK2107), so use `SharedMut`; and `scope ... catch` needs `fn main() throws`.
   - Where the checker reads std declarations (`std.contracts`, built into the binary).
3. **Checks I ran:**
   - The `nikaia-std` `task::` unit tests and the temporary Rust probes (removed afterwards): useful, they were the only way to exercise the handles.
   - The `tests/compiler` run, the `it` filters, the guards, clippy and pre-push: all needed by the brief.
   - Wasted: running the test programs on the old binary first proved little, since behaviour was meant to stay the same. The guard run (95 s) touched nothing I changed. I also ran clippy after the full test run, when running it first would have caught the lint before the long run.
4. **The change that would save the most:** a short "surface today" note in briefs that change a runtime. It would say which pieces Nikaia programs can reach now and which wait for the lowering (here: only `task::scope`; handles wait for #988). It would also say how to test what only shows on stderr (a program test needs exit 0; secondaries show only in an uncaught error, so use the `it` test `process_exit`). That would have saved both the design detours and the test search.
