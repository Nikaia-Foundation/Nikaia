Result: done
Files: crates/nikaia-std/src/tools/buffers.nika crates/nikaia-std/src/tools/lowered/buffers.rs tests/compiler/tests/a_view_of_a_concatenated_local_is_returned_owned.nika tests/compiler/tests/a_view_of_a_concatenated_local_is_returned_owned.stdout
Commit: A local built with + is text of the frame's own: a view of it leaves through the keep
Fixes: #864
Tests: tests/compiler both settings 109 ok; guard set, tether_keep, views_handed_back, touches, keeps, tethers ok; clippy clean. New test failed on the unfixed compiler (rustc: Cannot return value referencing local variable `s`)
Gate: green
Deviation: root was not text_tiers (tier was already View) but buffers.nika made_here/makes_a_buffer: `text + text` was no buffer, so the local was not put in the keep; fix is joins_text in made_here. Not fixed: `let s = t.len().to_string(); return s.trim()` (same symptom, Buffer::Unknown)
Found: `to_string()` of a non-literal (e.g. t.len().to_string()) as a let local also returns a view of it with rustc error; makes_a_buffer gives Unknown
Needs: none
Changelog: A view of a local built with `+` can be returned from a `String` function; the local lives in the caller's keep (#864)
Owner: none
Disk freed: 0
Pushed: 6ba5c7482101f3e9810b99322637eff48b1ff368
