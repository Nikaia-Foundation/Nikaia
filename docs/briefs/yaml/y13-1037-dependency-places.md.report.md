Result: done
Files: crates/nikaia/src/emit/mod.rs tests/compiler/src/main.nika tests/compiler/nikaia.contracts tests/compiler/nikaia.derived tests/compiler/tests/a_dependencys_type_with_two_view_fields_keeps_its_places.nika tests/compiler/tests/a_dependencys_type_with_two_view_fields_keeps_its_places.stdout
Commit: emit: a dependency's struct with two view fields keeps its places where it is named (ADR-353 D7)
Body: The reading program's own ledger holds a dependency's types under their written path (`compiler_tests::Places`) with that package's `tethered`, but `EmitState.view_slots` was filled only from this package's units and the described crates, keyed by bare name, so `lifetime_args` missed and wrote one lifetime. The emitter now also reads the places of every qualified type in its own ledger (`emit_tables_read_view_slots(&own_contracts)`, keys with `::`), before the units' own (bare) entries. The other three rustc errors of the issue followed from the first. tests/compiler/nikaia.contracts and .derived are the test package's ledger, regenerated for the new `pub` items.
Fixes: #1037
Tests: new a_dependencys_type_with_two_view_fields_keeps_its_places failed on the unfixed binary (rustc: Struct takes 3 lifetime arguments but 1 was supplied) and passes; tests/compiler --both-settings 418 passed; packages/yaml 10 passed; emit:: unit tests ok; guard set 24 passed
Gate: green
Deviation: none
Found: none
Needs: supervisor rebase and push of 423ab3fb (push rejected, not rebased per rules)
Changelog: A dependency's struct with two or more view fields, named as a type in the reading package (`n: ref two::Node`), is lowered with one lifetime per view field (ADR-353 D7), as in its own package
Owner: none
Rs2nika: none
Pushed: 423ab3fb (pushed by drive.nika)
Disk freed: probe targets in scratchpad; target/nikaia-project-tests 6.1G, 5.7G free on /
