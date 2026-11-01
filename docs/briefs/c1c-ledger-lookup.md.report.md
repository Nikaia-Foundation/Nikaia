Result: blocked
Files: none
Commit: none
Fixes: none
Pushed: no
Tests: built the function and forward; rustc refused the forward (E0308 then lifetime error); nothing committed, my edits reverted, lower-std rerun
Gate: green
Deviation: not built
Found: the lowering gives `ledger_items_lookup(ledger: ref Ledger, name: String) -> (String, ref FnContract)?` ONE lifetime `<'r>(&'r Ledger, &'r str) -> Option<(String, &'r FnContract)>`; the owned `String` in the result is `name.to_string()`, yet `name` is tied to the result's borrow. `check --tethers` says result is "borrowed from `l`, `name`" (name.clone() of a view param stays a view). Rust `lookup(&self, name:&str) -> Option<(String,&FnContract)>` then fails: "lifetime may not live long enough". Probe: scratchpad c1c/p.nika (tethers output)
Needs: architect decision (result should borrow only `ledger`: `name.clone()` of a text param yielding owned String in an owned result position, or per-parameter lifetimes in the signature)
Changelog: none
Rs2nika: none (draft was a faithful 1-line match, unused)
Attribution: Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
