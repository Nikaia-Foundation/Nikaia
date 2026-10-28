1. **What cost the most:** pinning down the two defects. The NK2605-after-`close()` probe took about 8 calls, because the `cleanup_points` `it` test looked as if it covered the pattern. In fact it lowers without the checker. The `&'r str` lowering probe took about 3 more. Next came the shared checkout: another agent renamed an untracked test and rebuilt `target/fast/nikaia` while my run was going, so `tests/compiler` had to run three times.
2. **What I looked up that the brief could have handed me:**
   - The calling shapes: how `comptime` constants lower (`pub(crate)`, so Rust can't see them), how `String?` and `fn(..)` parameters lower (`&Option<String>`, `&impl Fn`), and that `Manifest.build` is private.
   - That `std::fs::open` exists with `append`.
   - The `refuse!` versus `anyhow!` error mapping.
   - That a text slice needs `.bytes[..].text()`, which throws.
3. **Checks run:**
   - Useful: lower-std with the old binary (twice, caught refusals in seconds), one `cargo check`, one build cycle, and one gate run (lib tests, the `it` filters plus the guard set, clippy, `tests/compiler`).
   - Wasted: two `tests/compiler` runs killed by the other agent's changes, and an early `nikaia check` on a single tools file, which can't resolve the tools-wide names.
4. **The one change that would save the most:**
   - Flag drafts that come out as methods. The draft was method-shaped (`impl Settings`) although the brief said "free functions over words". At 6 % kept, it was close to useless.
   - Either convert with a free-function profile, or have the brief say "the draft's signatures are wrong; keep only the bodies".
   - Also, builder.md should tell testers to copy `target/fast/nikaia` to their scratch directory before running long tests, because other agents rebuild it in the shared checkout.
