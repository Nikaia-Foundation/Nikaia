Result: blocked
Files: none
Commit: none
Fixes: none
Pushed: none
Tests: preconditions on origin/develop (k3a1 5bf28f57, #995 0cf0b173, #997 7029fe65, #998 f2599c1c, #1005 8a30f8cb, #1006 03646a20/d2986cd9); step 2 written (trait ComptimeComputeHost, enums ComptimeDriven/ComptimeRan, struct ComptimeDeclared, comptime_computed, comptime_worked_out_ahead, the four consts) and lower-std accepted it, but the lowered Rust cannot compile (probe in Found); the draft is kept at /tmp/claude-0/-home-user-Nikaia/8a3a29dc-69fc-49cd-abeb-78092aa24a1f/scratchpad/k3a2/k3a2-nika.patch, my files restored by name, nothing committed
Gate: none
Deviation: none
Found: a `fn` parameter that lowers as `nikaia_std::func::Kept` (handed to a body-less trait method's `fn` parameter, which counts as keeping it) is moved on its first hand-off and never cloned, so a second use fails at rustc "use of moved value" while the checker accepts it. Probe: `trait H { fn ask(ref self, known: fn(ref String) -> i64? sync) -> i64? sync }` / `fn twice[T: H](host: ref T, known: fn(ref String) -> i64? sync) -> i64 { let a = host.ask(known) ?? 0; let b = host.ask(known) ?? 0; return a + b }` / `fn main() { println("x") }`: `nikaia run` gives "Use of moved value: `known`" at the second line; lowered `host.ask(known)` twice, a `.clone()` is missing (Kept is Clone). comptime_computed hands `known`/`typed` to `sub_program`, `unit_without_its_constants` (in a loop), `folded` and on to itself, so it meets this at every step. Also met: a trait method with a `fn` parameter implemented in Nikaia is refused "Method `ask` has 1 type parameter but its trait declaration has 0" (not needed here: the host is Rust); `ref fn(..)` as a parameter type does not parse, so no lent callback through a trait
Needs: fix of the Kept clone (a `ready` issue with the probe above), then a fresh k3a2 run from the saved patch; note for that run: a let-bound lambda handed to a Kept parameter needs its type on the `let` (`let known: fn(ref String) -> BuildValue? sync = fn(name) {..}`, as b997's commit says), `not_here` likewise typed `fn(ref String) -> CheckComputed sync`; trait text parameters `ref String` (builder-facts, host traits); `grammar` is reserved, so ComptimeRan::InsideAnother's field is `name`. docs/selfhosting-plan.md K3a row still holds another session's uncommitted part-1 edit
Changelog: none
Owner: none
Disk freed: 0 GB; cache 3.8G, disk 4.2G free; no new cache entry over 100 MB
Rs2nika: none
