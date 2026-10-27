Result: done
Files: crates/rs2nika/convert/src/main.nika crates/rs2nika/convert/nikaia.contracts crates/rs2nika/convert/nikaia.derived crates/rs2nika/convert/tests/host_b.stdout crates/rs2nika/convert/tests/if_let_narrow.in/f.rs crates/rs2nika/convert/tests/if_let_narrow.stdout docs/selfhosting-gaps.md docs/specification/adr/README.md docs/briefs/b869b-path-rest.md.step1.report.md docs/briefs/b869b-path-rest.md.step2.report.md docs/briefs/b869b-path-rest.md.step3.report.md
Commit: rs2nika: `if let Some(x) = &self.f` becomes `if self.f != null` with `x` read as `self.f`; the `let` stays only where the block writes the field (ADR-382 D7); R48 leaves a non-name pattern with code before it to R23. Not yet: `else if let`, let-chains and `&& let` in one-line closures stay TODO. Field row of selfhosting-gaps removed, ADR-382 built
Tests: rs2nika convert nikaia test 39 ok; guard set 22 ok; pre-push-check ok
Gate: green
Fixes: #869
Changelog: none
Model: Opus 5.5
Pushed: 65e011ee320dea0726a952d14adb801c0d021f28
