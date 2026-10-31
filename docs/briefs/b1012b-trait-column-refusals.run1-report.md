Result: blocked
Files: none
Commit: none
Body: none
Part: #1012
Pushed: no
Tests: no build; probes only (scratchpad/b1012b/p1/lend.nika, lend_local.nika)
Gate: not run (nothing to commit)
Deviation: stopped before step 1: the brief's premise does not hold, and the plumbing needed lies outside the allowed edit set
Found: (1) the contract pass raises no NK refusal today (no NK12xx raised under contracts/), so NK1264 needs a new path keeps.rs -> sync::Noted -> modules.rs Program -> diagnostics::refuse; the impl that causes NK1265 can reach contract_changes::changed_contracts only through project.rs:1217 (its only caller, via project_lower.nika host.changed_contracts); the lending note needs the keeper map in check/mod.rs Newly -> CheckWorld (check_state.nika) and a callee for method calls, which pass `callee: null` to check_core_arguments_given (check_core_call.nika 176/427/761). modules.rs, project.rs and check/mod.rs fall outside the brief's list. (2) A defect: a fn *parameter* `s: String` lent to a trait method whose impl keeps the callback (`fn hand[T: H](mut h: T, s: String) { h.take(fn(x){ x + s.len() }); println(s) }`, with impl `self.held.push(f)`) gives a raw `error: Lifetime may not live long enough.` at the call, with no NK code. The same code with `let s = f"a{t}"` is refused correctly with NK2101. The refusal for a kept trait-method argument does not reach parameters (a rustc error leaks through); file it as `ready` with this probe.
Needs: owner decision: widen #1012 step 2's edit set to modules.rs (Program field + refusal), project.rs (pass the trait-method -> impl map to changed_contracts and Newly) and check/mod.rs (Newly.keepers), or name another route; fix defect (2) first, since test (h)'s lending caller relies on it
Changelog: none
Owner: decide the edit set above; issue comment for #1012: "step 2 blocked: NK1264/NK1265 need plumbing through modules.rs/project.rs/check/mod.rs; lending a String parameter to a keeping trait impl leaks a rustc lifetime error (probe in report)"
Disk freed: 0 (see final answer for figures)
Rs2nika: none
