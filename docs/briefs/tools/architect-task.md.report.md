Result: done
Batch: j3-1019-meta-field.md j4-1019-members.md
Owner: none
Decided: `meta::Field` names a reflected field's type; a parameter of it is `comptime` by its type (a copy per field, ADR-specialization-1 D1), handed over without a move; elsewhere `NK1320` (Part II 10.3, ADR-grammar-2 D4 amended, Part III). Left out, `members[S]`'s `name:` is the field's name, `null:` matches nothing, `unknown:` skips nothing (Part II 10.8). Both by *Language decisions: obvious*. Spec text changed: the supervisor raises the version.
Filed: #1083 (a bare variant name in a pattern, `Name(v)`, reaches rustc instead of NK1252; `ready`, no `wip`, for the bug lane).
#1019: steps 1-2 ticked, step 3 split 3a-3d; labels `language,ready,wip` (was `blocked`).
j2's Needs (`f[i64?](x)`): already #1079, bug lane.
Next batch: 3c (`name:` evaluated per field while building, NK1311/NK1312; designed on j3's copies), 3d (ParseError path, string views).
Process: builder-facts *Traps already met* gains j2's two (no rebuild of `target/fast/nikaia` while `tests/compiler` runs, one program in a scratch project; kind names and `else`).
Unfiled, seen while probing: `f"{x ?? \"-\"}"` is refused ("`??` needs a fallback"); Part II 12 writes a fold step as `fn(ref mut A, T)` while a declared `ref mut` parameter is NK1158 (spec wording check).
Pushed: rejected (local c07c79f before this line; the driver rebases)
Needs: scripts/pre-push-check.sh fails on check-adr-refs (ADR-384 has no index row: the main lane's s1082 record, not this lane's); left for the supervisor.
