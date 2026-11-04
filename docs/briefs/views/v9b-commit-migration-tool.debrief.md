1. **What cost the most:** little, since the work was already in the tree. The steps that took longest:
   - `nikaia test` has no `--filter`. I had to read `--help` and then build a scratch project holding only the two tests.
   - `nikaia fmt --check` flagged every file. Comparing it on a scratch copy showed that existing `tools/*.nika` files fail it too, which `docs/builder-facts.md` explains (4-space convention).
   - The first `pre-push` run failed on the self-hosted badge, and I had to look up how to regenerate it from the staged files.
2. **What the brief could have handed me:**
   - That `nikaia test` has no `--filter`, and how to run a few tests instead (a scratch project with a copied `nikaia.toml` and the test files).
   - That a binary without the tool already exists for the "fails without it" check: `target/release/nikaia` (Nov 2). It fails on the unknown subcommand, so the check is real but weak.
   - That `fmt --check` is expected to flag `tools/*.nika` and the long-line tests.
   - The badge regeneration recipe, given up front.
3. **Checks:**
   - Ran: `lower-std` → build → `lower-std` (the lowered files didn't change, so that build only confirmed the tree), `fmt --check` plus a scratch diff, the two tests on the new and the old binary, `cargo fmt`, clippy, `--lib fmt::`, and `pre-push` twice.
   - Wasted: `--lib fmt::` ran 0 tests (wrong filter for that module), and the first `fmt --check` run.
   - Not run: `gate.nika`. The system rules give the full suite to the driver, so the report's `Gate: green` means the cheap checks only. I said so under `Tests:`, but the field itself overstates it.
4. **One change:** the brief's lines "Run them with `nikaia test tests/compiler --filter migrate_views3`" and "Run the full gate" contradict the tool (no `--filter`) and builder.md (the driver runs the gate). Give the real way to run a few tests, and replace `Gate: green|red` with something like `Gate: cheap-checks green`. That removes both the detour and the misleading field.
