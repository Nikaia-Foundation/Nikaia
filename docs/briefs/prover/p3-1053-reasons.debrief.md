1. **What cost the most:** learning the AST and walker API that Nikaia tools code uses. That took about six read turns: the `Stmt`, `Expr`, `FnArg` and `BinaryOp` shapes, `id_of`, the `check_visit_*` helpers, `written_expression`, and the fact that the interner is `parsed.interner`. After that, one Rust borrow conflict cost a rewrite before the first build: calling `self.blocked_*` inside the `visit_stmt` closure clashed with `self.seen`. The build cycle went through on the first try.
2. **What the brief could have handed me:**
   - That `tools/check_visit.nika` (`check_visit_preorder`, `check_visit_stmt`, `check_visit_stmt_blocks`) is the walker to reuse.
   - That `id_of` in `ast.nika` gives a node's id.
   - That `written_expression(words, e)` prints an expression as written, and Rust passes the interner as `&parsed.interner`.
   - That `bounds_basic.nika` already shows a similar per-statement binding walk (`statement_says`, `any_symbol`).
   - Which `it` modules run `--bounds`; I found them with a grep.
3. **Checks run:**
   - Each paid off: one `lower-std`, build, `lower-std` cycle; the brief's probe with `check --bounds`; `lower-std --bounds` with `proved.nika`; seven `it` modules (65 tests); `cargo fmt`; clippy on 1.97 and on 1.99.1 (installed, cold build in a separate target dir, then deleted); `pre-push-check.sh`, then the badge.
   - Wasted: `cargo test --lib bounds_report::` (0 tests exist), and the grep of three `it` files that matched only on the word "checked: ".
   - The 1.99.1 clippy was the most expensive check. It was needed only because of the toolchain gap, and found nothing.
4. **One change that would save the most:** a builder-facts entry, "Walking a body in `tools/*.nika`", listing those helpers (`check_visit_*`, `id_of`, `written_expression`, `words.resolve`) with a five-line example. Also, a brief that adds parameters to a Nikaia function called from Rust could say "build a small Copy context struct before the closure", which would avoid the borrow trap.
   - Second: preinstall CI's stable toolchain with clippy on the machine, so no builder pays for the cold 1.99.1 build.
