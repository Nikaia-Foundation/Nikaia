Result: done
Files: packages/yaml/src/main.nika packages/yaml/tests/anchors_tags_aliases.nika packages/yaml/tests/anchors_tags_aliases.stdout packages/yaml/nikaia.contracts packages/yaml/nikaia.derived packages/yaml/nikaia.lock
Commit: yaml: anchors, tags and aliases (y2 step 2)
Body: Properties as `struct Props { start, end, anchor, tag }` built by `rule Props`, copied onto the Node in Propped and Content; an alias is a `Value::Alias` node, not resolved (ADR-yaml-1 D6). A core tag hands a TagKind to the scalar rules, which fix the kind and check the scalar's shape in the grammar with a guard (`fits_node`, `empty_fits`), so `!!int "x"` is a parse error at its position (D5, D11). Local tags, verbatim `!<..>` and named handles `!e!x` are kept as written (`%TAG` resolution is y2b). Properties above a block collection and on an empty node. Built by run 2 (wip/y2c-properties b223f972) on the y2 draft ea068fa4; run 3 applied it on develop after #1015's fix and formatted it with nikaia fmt. Expected output checked by hand against YAML 1.2.2 (examples 2.10, 2.24, 6.23, 6.24, 6.28, 7.1, 7.2).
Fixes: none
Part: #928
Pushed: 95a690feb3278aca008f92038187be123fb76485
Tests: pre-flight: winnow-grammar rev present, fmt_keeps_a_grammar_guard_whole on develop, binary up to date (cargo build fast was a no-op); nikaia fmt on both files then nikaia check passes; nikaia test --project packages/yaml 11/11 (new anchors_tags_aliases included); fmt::every_nika_file green
Gate: green
Deviation: none
Found: none
Needs: the commit message of 95a690fe reads `Fixes none` instead of `Part of #928` (my report had `Fixes: none`; scripts/commit.nika prefers any non-empty Fixes over Part and does not skip `none`, as it does for Body/Changelog): link the commit to #928 by hand; commit.nika could skip `Fixes: none`
Changelog: none
Owner: none
Disk freed: 0 GB (target/nikaia-project-tests 6.1G, / 8.2G free; no cache entry over 100 MB new; no clone)
Rs2nika: none
