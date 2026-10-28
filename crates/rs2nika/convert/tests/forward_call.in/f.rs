/// The name the `k`th test's function has in a test build.
fn test_function(k: usize) -> String {
    nikaia_std::tools::modules::test_function(k as i64)
}

fn second_test() -> String {
    test_function(2)
}
