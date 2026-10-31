Result: blocked
Files: packages/yaml/src/main.nika packages/yaml/tests/block_scalars.nika packages/yaml/tests/block_scalars.stdout packages/yaml/nikaia.contracts packages/yaml/nikaia.derived packages/yaml/nikaia.lock packages/yaml/nikaia.proofs
Commit: yaml: block scalars, literal and folded, chomped and indented (YAML 1.2.2 8.1)
Part: #928
Pushed: rejected (step 1 committed as 9fe7210f on local develop; git push origin develop rejected twice as non-fast-forward, not rebased, per the brief)
Tests: step 1: packages/yaml 10/10 (new block_scalars: chomping with and without trailing empty lines, no text line, no final break, example 8.10 folding, more-indented lines, |2 with a leading space line, both indicator orders, sequence, top level, after ---, four header and indentation errors; it is a new feature, so it cannot pass on the old grammar); fmt::every_nika_file green; pre-push-check clean; no crates/ changed, so no clippy
Gate: green
Deviation: step 2 (properties and aliases) not built: blocked on the defect under Found; its draft is on branch wip/y2-properties (ea068fa4), and it does not check
Found: a view read out of a struct that a grammar sequence binds cannot go into the struct the rule returns, NK1106 `Pair.a holds String, but you're giving it ref String`; after that, the field refuses even a direct text view. Probe: `struct Props { a: String }`, `pub struct Pair { pub a: String, pub b: String }`, `entry rule top -> Pair = x:text(Word) " " y:text(Word) eof -> { Pair { a: x, b: y } } | p:P " " y:text(Word) eof -> { Pair { a: p.a, b: y } }`, `rule P -> Props = "&" x:text(Word) -> { Props { a: x } }`: both alternatives give NK1106. Without the second alternative it runs. The same happens with String? fields, with `let an = p.a`, and with `match p { Props { a: x } => .. }`. Moving a Value out of a bound Node (`value: v.value`) is accepted
Needs: compiler fix of the defect under Found (a ready issue: a view field read from a grammar-bound struct is a view of the input, ADR-282 D12-D13), then y2 step 2 from wip/y2-properties; step 1 still needs pushing (supervisor rebase)
Changelog: none
Owner: none
Disk freed: 0.0 GB (target/nikaia-project-tests 5.8G, 11G free on /; new cache entry over 100 MB: run-bundle/b2db8b4e959f1cfc 341 MB)
Rs2nika: none
