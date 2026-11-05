Result: done
Files: crates/nikaia-std/src/task/group.rs crates/nikaia-std/std.contracts crates/nikaia-std/src/tools/check_core_method.nika crates/nikaia-std/src/tools/check_core_call_on.nika crates/nikaia-std/src/tools/lowered/check_core_method.rs crates/nikaia-std/src/tools/lowered/check_core_call_on.rs tests/compiler/tests/async_join_within_child_keeps_the_task.nika tests/compiler/tests/async_join_within_child_keeps_the_task.stdout tests/compiler/tests/async_join_within_child_cancel_stops_the_task.nika tests/compiler/tests/async_join_within_child_cancel_stops_the_task.stdout tests/compiler/tests/async_join_within_child_in_time.nika tests/compiler/tests/async_join_within_child_in_time.stdout
Commit: task::ChildHandle takes join(within:, on_timeout:) as TaskHandle does (Part I 8.2, ADR-async-5 D17)
Body: `h.join(within: d)` on a `task::ChildHandle` throws `task::TimedOut` on time-out and leaves the task running and the handle live (joined again or cancelled); `on_timeout: task::OnTimeout::Cancel` cancels it as `cancel()` does and uses the handle up. The two checker predicates that key `join(within:)` on the receiver (`check_core_method_task_handle`, `check_core_call_on_task_handle`) take `task::ChildHandle` too, and std.contracts describes `task::ChildHandle::join(within)` and `join(within, on_timeout)`; the emitter needed no change (`join_within(d, on_timeout)` was already keyed on the name). `NK2109` is not asked of a child handle: Part I 8.2 waits for one left live at its group's end. Rust changed (#1214, for #557/#125), all in crates/nikaia-std/src/task/group.rs: new `ChildHandle::join_within(&self, span, on_timeout) -> Result<T, Box<dyn Error>>` (it lends the handle, so it takes `&self` like `TaskHandle::join_within` and the emitted `let h` needs no `mut`; the error is boxed because a plain group's channel `Crashed` cannot hold `TimedOut`); new `ChildHandle::cancel_here` (the body of `cancel`, on `&self`); `ChildHandle` fields `claimed` now `Cell<bool>` and `held` `RefCell<Option<…>>`; `cancel`, `join` (now `self`, not `mut self`), `join_now`, `held` and `Drop` adjusted to them.
Part: #987
Tests: tests/compiler async_join_within_child_{keeps_the_task,cancel_stops_the_task,in_time} new (keeps and cancel refused NK1103 by the compiler before the change); tests/compiler async_ task_ spawn_ overlap_ --both-settings 65 passed 0 failed; nikaia-std --lib task:: 7/7; clippy clean; lower-std fixpoint ok
Gate: green
Deviation: none
Found: a pausing `async(handle)` task whose handle is dropped live (here after a Keep time-out the handler does not take up) is cancelled at its pause, not waited for at its group's end as Part I 8.2 says: the rest is held in the handle (`ChildHandle::held`, a14) and may borrow what lives shorter than the group, so the drop can only cancel it; probe: `fn slow() -> i64 { time::sleep(100.millis()); println("end"); return 7 } fn main() { let h = async(handle) fn { slow() }; let v = h.join(within: 10.millis()) catch { 0 }; println(f"{v}") }` prints `0` without `end`
Needs: supervisor decision
Changelog: `join(within:, on_timeout:)` works on a `task::ChildHandle` as on a `TaskHandle`: on time-out the task keeps running and the handle stays live, or with `task::OnTimeout::Cancel` the task is cancelled.
Owner: none
Disk freed: none (cache 2.7G, / 4.4G free; no new entries over 100 MB)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 3fea55c95ab3b88bfecdbb1a0e9e7bd662715f96
