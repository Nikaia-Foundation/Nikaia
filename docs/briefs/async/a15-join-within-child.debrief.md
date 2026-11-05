1. **What cost the most:** choosing the shape of `join_within` in the runtime. My first version took `&mut self`, and the compiler rejected it on a lifetime bound. Then I saw that `&mut self` would also force `let mut h` in the generated Rust. Rewriting `claimed` as a `Cell` and `held` as a `RefCell` took about five turns. Working out which error type the group uses (`Crashed`, `Box<dyn Error>` or a typed one) took another three.

2. **What I looked up that the brief could have handed me:**
   - That 8.2 never asks `NK2109` of a child handle: "a child handle left live is waited for at its group's end". The brief's "`NK2109` where 8.2 gives it" sent me to read the spec, the ADR and `check_crossings`.
   - That Nikaia has no `?` for passing errors on. My probe hit a parse error.
   - That `group_paused_plain` gives a group the error type `Crashed`.
   - The emitter already wrote `join_within` for any receiver.

3. **Checks I ran:** the old-binary probes (keep, cancel and drops all refused with NK1103, which was the expected starting point); one build cycle; the probes at both settings; the `async_`, `task_`, `spawn_` and `overlap_` tests at both settings (65 passed); `nikaia-std` `task::` unit tests; clippy; pre-push.
   - **Wasted:** the `?` probes (q, q2, q3) and the `async e` probe (held.nika). That probe was refused with NK2110 because the `async` had nothing running beside it.

4. **The change that would have saved the most:** a *Facts* line in the brief saying:
   - a child handle is exempt from `NK2109` (8.2, last sentence);
   - Nikaia has no `?` for errors;
   - a runtime method that lends the handle takes `&self` (as `TaskHandle::join_within` does), because the generated `let` has no `mut`.

   The brief also hinted that "`join` takes `&mut self`", which was out of date (it takes `self`) and misleading.

(Since the commit, `group.rs` has changed on disk, presumably another agent's edit. I didn't check it.)
