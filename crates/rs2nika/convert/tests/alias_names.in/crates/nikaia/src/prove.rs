pub use nikaia_std::tools::prover_results::{
    ClaimHeld as Held, Published,
};

fn tail_of(block: &Block) -> Option<&Expr> {
    nikaia_std::tools::check_types::tail_value_of(block)
}
