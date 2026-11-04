Result: blocked
Files: none
Commit: none
Pushed: none
Tests: probes only, old binary (no build): views-3 (a) in one function -> rustc "Mismatched types"; (a) returned -> same; (b) `go(l: L)` -> NK2105, ledger `keeps = ["l"]`; with the field written `ref L` (stand-in for an inferred view field) `let first = xs[0]` into it runs (prints 5), `go(l: L)` and `run(config: C)` with `config.part` still NK2105 (keeps), `W { own: xs[0] }` is NK1106 (check_core_places_a_local_lent takes Variable/Field, not Index), a literal `L {..}` into it NK1106, a returned one rustc "Cannot return value referencing local variable"
Gate: green
Deviation: steps 1 and 2 not built: step 1's `go` rule needs a change in keeps.nika (a stop condition of the brief), and step 2 builds on step 1
Found: none
Needs: keeps lane (keeps.nika `classify`, the `Expr::StructLit` arm, and `keeps_a_lent_text`, line ~1024): "a struct literal keeps every field it is given" lends only a lent parameter whose type is `String` (`ty == "String"`). #1218 (b) / D5's last paragraph need it to lend a lent parameter of any type (and its parts, `config.part`) that flows into a field holding a view. Probe: `struct L { n: i64, tag: String } struct W { own: L, k: i64 } fn show(w: W) -> i64 { return w.own.n + w.k } fn go(l: L) -> i64 { let w = W { own: l, k: 1 }; return show(w) } fn main() { let l = L { n: 3, tag: "a" }; println(f"{go(l)} {l.tag}") }` gives NK2105 even with `own: ref L`. Once that lands, v5 can go on: plan in the next field.
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Body: Plan for the rerun (all inside v5's lane): (1) at parse time, beside `tier_write` (text_tiers.nika, called from crates/nikaia/src/text_tiers.rs `refine_with`), infer for each field whose declared type is a non-text struct whether a view flows into it from a struct literal (`xs[i]` of a place, `a.b` of a place, a `let` bound to one of those, a lent parameter) and mark the field type `is_view`, as a written `ref L`; the existing #1152 path (check_core_places.nika ~line 629, `lent_fields`, emit_core_places.nika ~line 642 writes `&`) then lowers `own: &'a L`, the struct gets its lifetime and the ledger `tethered = ["own"]` (seen with `ref L`). (2) extend `check_core_places_a_local_lent` with `Expr::Index` so `W { own: xs[0] }` is lent. (3) an owned value into such a field (literal, call result) needs a decision: text uses `EitherText` (a Cow, crates/nikaia-std/src/either_text.rs, text-only) for a mixed field, and a keep (`Keep::put`, generic in crates/unsafe/tether) for a returned one; a non-text mixed field would need a generic Cow-like std type, which is new Rust under the #1214 exception. (4) step 2: buffers.nika's plan must put a returned view's non-text local (`xs`) into the keep as `put_viewed` does for text (`make(__keep)` in the text probe).
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
