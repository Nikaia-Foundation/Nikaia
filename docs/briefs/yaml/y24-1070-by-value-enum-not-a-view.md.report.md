Result: done
Files: crates/nikaia-std/src/tools/views.nika crates/nikaia-std/src/tools/lowered/views.rs tests/compiler/tests/by_value_enum_beside_a_view.nika tests/compiler/tests/by_value_enum_beside_a_view.stdout
Commit: views: a by-value parameter that holds no text is no view source
Body: The result-source walk (`rsrc_function`, views.nika) marked every by-value non-scalar parameter `!` (owned by the call), so a function that put a field-less enum into a view-holding struct was refused NK2302. The new `rsrc_textless` takes a scalar, or a struct or enum the unit declares whose fields are all such types (recursively, a recursive type is not), as holding nothing a view could point into; such a parameter is no longer marked. A type declared outside the unit, a tuple, a generic, `String` or a struct with a `String` field stays marked, so NK2302 stays where a view of it is handed back.
Fixes: #1070
Pushed: local 1ea52a920ad6800702611226982cc6455f8cad36
Tests: new tests/compiler by_value_enum_beside_a_view prints `hello hello 2` (NK2302 on the unfixed suite binary); tests/compiler --both-settings 479 passed; packages/yaml 16 passed; it text_views stored_views tethers views_kept result_source keeps accepted fmt::every_nika_file 191 passed; nikaia-std --lib 239 passed; clippy 1.97 clean, 1.99.1 not run (7.4 GB free disk, lowered change only)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A by-value parameter whose type holds no text (a field-less enum, or a struct or enum of only such fields and scalars) is no longer taken as the source of a view handed back, so building a view-holding struct from it is no longer refused NK2302.
Owner: none
Disk freed: 0 GB
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
