Result: done
Files: packages/yaml/src/main.nika packages/yaml/nikaia.contracts packages/yaml/nikaia.derived packages/yaml/tests/scalar_written.nika packages/yaml/tests/scalar_written.stdout
Commit: yaml: a scalar node keeps its text and style as written (ADR-yaml-1 D7)
Body: Node gets pub raw: String and pub style: Style. Every scalar rule (Plain, Single, both Double branches, BlockScalar) sets them from the view its Scalar.raw takes (between the quotes, escapes not decoded; after a block header), whatever the Value; collections, aliases and empty nodes (also after properties, !!null and !!str alone) get "" and Plain; a node taking properties copies raw and style from its content, as it copies flow. Views only, nothing copied. The test's .stdout was blessed from the run and checked line by line against the brief's step 1; the test's style printer takes ref yaml::Style (Style is not copied out of a borrowed node).
Part: #928
Pushed: local ff39fc6f551fc8ecdd78eeff14c730b0119d921c
Tests: nikaia test --project packages/yaml 16/16 (15 old byte for byte, scalar_written new); against the unchanged package the test cannot compile (Node has no raw/style), not run; fmt::every_nika_file ok; pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: none
