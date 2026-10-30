1. **What cost the most.** The pre-flight probe took four tries:
   - A grammar entry returns a throwing result, so `match` on it fails; it needs `catch`.
   - A `text(..)` view must be cloned when the rule returns `String`.
   - A `match` needs an `else` arm.

   Next came the error columns. I spent two turns finding out that a failed guard reports the position after the element it read, and that a cut does not move it.
2. **What the brief could have handed me:**
   - That develop already has `hex_value` from the `0x`/`0o` work, so the draft adds a second one (NK1148).
   - That `DoublePart -> String` cannot return the `t`/`w` view from `text(..)`; it needs `.clone()`.
   - The grammar entry's call form, `G::top(t) catch { .. }`, for the probe.
   - The cut syntax (`p => q`).
   - That a guard's error column is the position after what it read, so the expected `.stdout` columns are known in advance.
3. **Checks:**
   - Useful: the scratch probe with `check --asserts`, `check --asserts` on the package after each edit, the package tests (once failing, once with `--bless`, once after fmt), `fmt::every_nika_file` and `pre-push-check`.
   - Wasted: the cut probe (it changed nothing), and arguably the test run before `--bless`, since the new tests had no expected lines yet.
   - Skipped: clippy, since no Rust changed.
4. **One change that would have saved the most:** a ready-made probe skeleton in the brief (an entry call with `catch`, a rule returning a cloned `text(..)`), plus a line saying "develop already defines `hex_value`/`is_hex`; drop the draft's copies". That would have saved about five turns.

The report's `Tests:` line was rewritten on disk: `commit.nika` turned `\u00e9`/`\u0020` into the actual characters. The meaning is intact, but the escapes no longer read as written; that is a small defect in `commit.nika`.
