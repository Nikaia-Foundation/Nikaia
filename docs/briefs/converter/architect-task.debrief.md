1. **What cost the most:** hunting for where the `let mut (a, b)` parse error is raised. I spent about six greps, including one in the cargo registry and one that timed out into the background, and one of them dumped a 100 KB file into my context (`probe.in/out.lsif`). I never found the source; I gave up and wrote "not traced" in b29. Second: I probed the converter by hand to learn which patterns it already handles, and several of them (if-let, let-else, `matches!`, the `?? continue` rule) turned out to work already.
2. **What the brief could have handed me:**
   - That the converter binary sits at `crates/rs2nika/convert/target/nikaia/bin/rs2nika` and the compiler at `target/fast/nikaia` (the compiler is not on PATH).
   - That `docs/rs2nika-score.md` is out of date and the useful costs are the moves' `Rs2nika:` lines.
   - That `if let` is refused by design (Part I 2.3), so the converter's `match` form for it is correct.
   - The forms ADR-374 allows for cutting a text at a number (`bytes[..].text()`, `selfhosting-gaps` row).
   - The #886 rule, and the fact that a53f456's freeing of methods only applies when the input declares the struct.
   - The open S1 plan row, which named exactly this lane's work.
3. **Checks:**
   - **Useful:** converter probes on small Rust inputs and on the real S1 cut; compiler runs of every form the briefs prescribe. These found both defects and the cv1 rule.
   - **Wasted:** the message-text searches (about 6 calls); `where.nika` on the converter's names (it found 0 calls there); the generic `a.rs` and `c.rs` converter probes of already-working forms; the duplicate-issue search for NK1307.
4. **The change that would save the most:** the converter lane's `architect-task.md` (or `architect-input.txt`) should carry a short "converter state" block: the binary paths, the five costliest open patterns from the latest `Rs2nika:` lines with one probe each, and the plan rows waiting on the converter (S1, C1). The architect would then confirm and write briefs, not rediscover. Also add a line to `builder-facts.md`: `where.nika` does not index `crates/rs2nika/convert`, and parser error text is not greppable in the sources (find the rule by its outline).
