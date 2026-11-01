Result: done
Files: crates/rs2nika/convert/src/tr_expr.nika crates/rs2nika/convert/tests/text_cuts.args crates/rs2nika/convert/tests/text_cuts.in/f.rs crates/rs2nika/convert/tests/text_cuts.stdout
Commit: rs2nika: a text cut at a number goes through its bytes (ADR-374)
Body: tree engine (tr_expr.nika): `text_expr` types a text from params, typed or untyped lets (initialiser a text), String fields, cuts, trim/as_str/clone of a text and `unwrap_or_default` of a text option; `text_cut` writes the brief's rows (bytes cut + `.text()`, panic catch when the function does not throw, starts_with/ends_with on `.bytes`, `get(a..)` if-form, `[..]` -> `t`, find/rfind kept with the ADR-374 TODO); unknown and Vec<u8> receivers stay. S1 draft: all six lines changed (`text.bytes[at..].starts_with("impl".bytes)`, `text.bytes[copied..<at].text() catch {panic}`, `?? ""` and the get if-form, `rest.bytes[name..].starts_with(": ".bytes)`, `(text.bytes[at + 1..].text() catch {panic}).scalars.next()`, find with TODO); `nikaia check` on a scratch copy leaves NK1214 only on the find line. Corpus unchanged (attributes 76/440, defaults 49/187, function 9/49, program 32/137, seeds 53/115, all 219/928): no floor raised. Commit 72e7789c fixed the emitter defect that blocked building this (pausing cycle across package files).
Part: #125
Tests: convert `nikaia test` 146 passed (text_cuts new); pausing_cycle_across_files failed unfixed, passes (72e7789c); cargo test --lib emit:: ok; clippy, fmt, pre-push-check clean
Gate: green
Deviation: none
Found: `code_on_a_cycle` (emit_reach.nika, #1023) still builds its graph from one file only, the same blind spot fixed for pausing in 72e7789c; not met here
Needs: none
Changelog: rs2nika: a Rust text cut at byte numbers becomes a cut of the text's bytes (`t.bytes[a..<b].text()`, `get(a..)`, `starts_with` on bytes, `unwrap_or_default` -> `?? ""`), ADR-374.
Owner: none
Disk freed: 62M scratch; cache 2.6G, 5.8G free, no new entry over 100 MB
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local db8d83645361489c883a65936114582e9f5ed507
