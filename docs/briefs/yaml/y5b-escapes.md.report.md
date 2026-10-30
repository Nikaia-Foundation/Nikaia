Result: done
Files: packages/yaml/src/main.nika packages/yaml/tests/quoted_scalars.nika packages/yaml/tests/quoted_scalars.stdout packages/yaml/tests/errors_carry_line_and_column.nika packages/yaml/tests/errors_carry_line_and_column.stdout packages/yaml/nikaia.contracts packages/yaml/nikaia.derived packages/yaml/nikaia.lock packages/yaml/nikaia.proofs
Commit: yaml: double-quoted escapes checked and decoded once, in the grammar (ADR-yaml-1 D4, D11)
Body: Takes over wip/y5-escapes-in-the-grammar (72df62ae, applied three-way onto develop) and finishes it: Double reads a scalar with an escape or a line break in one pass, s:text(parts:DoublePart*) (#1013 point 1), raw the covered view and the value the decoded parts; DoublePart checks the digits of \x, \u, \U by a guard per digit and the range by a guard on the value, so all three scalar::from are proved. Scalar.value is private (ADR-282 D22) and read through text(), which returns it for Double and folds Plain/Single on request; the lazy decoding in fold, its escape table and the raw-text rule DoubleChar go, and the draft's second hex_value goes (develop's, from 0x/0o, is used). A plain run or blank run in DoublePart is cloned, since its parts mix views and decoded text. Steps 1-3 in one commit (the draft alone does not build). The four refused escapes report the column right after the element the failing guard examined (the closing quote, or after it for \u12), the toolchain's guard position; the .stdout is the real output. The package has no struct S, so #1013 point 2 is not met.
Part: #928
Pushed: b515702355caf4aa5d69fe7e83929f7482426d6a
Tests: packages/yaml 9/9 (quoted_scalars and errors_carry_line_and_column extended: \x41, é, é,  , " ", \U0001F600, escaped break before an empty line; \uD800, \U00110000, \x4G, \u12 with line and column; no yaml-test-suite case named, suite not in the checkout); check --asserts on packages/yaml: 3 scalar::from, all 3 proved; on develop before, quoted_scalars printed \x41é as written (old .stdout), the new lines were not run separately against it; fmt::every_nika_file green; pre-push-check clean; clippy not run (no Rust changed)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0.02 GB
Rs2nika: none
