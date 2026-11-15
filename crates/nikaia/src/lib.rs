// crates/nikaia/src/lib.rs
//
// The compiler front-end as a library, so integration tests (and later other
// tools) can drive it. The binary in `main.rs` is the CLI on top of this.
//
// **The parser's combinators are instantiated deeper than rustc's default
// limit of 128 allows** since the ledger inference and the bounds walk read
// conditions back through `parse_to_ast` (ADR-314 D3). At the `dev` profile
// (`opt-level = 1`) rustc 1.97 needs more than 256 for the grammar's closure
// (`parser/mod.rs`); the depth is finite (512 builds), so this is a limit, not
// a cycle: release and `fast` build at 256 too.
#![recursion_limit = "512"]

#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};

pub mod assets;
pub mod ast;
pub(crate) mod baton;
pub mod bounds;
pub mod bounds_report;
pub mod build_time;
pub mod cexport;
pub mod check;
pub mod comptime_run;
pub mod contracts;
pub mod describe;
pub mod diagnostics;
pub mod emit;
pub mod fixed;
pub mod fmt;
pub mod fold;
pub mod fold_bound;
pub mod foreign;
pub mod grammar_run;
pub mod interpreter;
pub mod libraries;
pub mod manifest;
pub mod migrate;
pub mod modules;
pub(crate) mod one_variant;
pub mod parser;
pub mod project;
pub mod proofs;
pub mod prove;
pub(crate) mod run_fast;
pub mod specbook;
pub mod sysroot;
pub mod text_tiers;
pub mod traits;
pub mod type_rules;
pub mod types;
pub mod view_changes;
pub mod views;

/// **This compiler's identity for its caches** (ADR-021 D3, #801, #1610):
/// the fingerprint `build.rs` takes of the sources it can read safely (the
/// compiler's Rust, the `.nika` files, the ledgers, rustc, `Cargo.lock`)
/// together with the digest `lower-std` wrote into the lowered set this
/// binary was compiled from (`LOWERED_DIGEST`). The lowered files are not
/// read by `build.rs`: in a shared checkout they can change between its run
/// and nikaia-std's compile, and the binary would be named after code it does
/// not hold.
pub fn compiler_identity() -> &'static str {
    static IDENTITY: std::sync::OnceLock<std::string::String> = std::sync::OnceLock::new();
    IDENTITY.get_or_init(|| {
        orchestrator::cache::sha256_hex(
            format!(
                "{}\n{}",
                env!("NIKAIA_COMPILER"),
                nikaia_std::compiler::LOWERED_DIGEST
            )
            .as_bytes(),
        )
    })
}
