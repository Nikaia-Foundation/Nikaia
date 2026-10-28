fn as_promised(mut ledger: Ledger, another: impl Fn(&str) -> bool) -> Ledger {
    for (key, contract) in ledger.functions.iter_mut() {
        if contract.sync_claim == Sync::Inferred && another(key) {
            contract.sync_claim = Sync::Unpromised;
        }
    }
    ledger
}
