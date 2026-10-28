fn top() -> i64 {
    helper(3)
}
fn helper(a: i64) -> i64 {
    a * 2
}
struct S { n: i64 }
impl S {
    fn outer(&self) -> i64 {
        let x = helper(1);
        fn inner(a: i64) -> i64 {
            a + 1
        }
        inner(x)
    }
}
