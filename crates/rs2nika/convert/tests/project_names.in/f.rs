pub fn name(node: Rc<RefCell<Node>>) -> &'static str {
    lookup(build_ctx, node)
}

pub fn label(t: &str) -> String {
    t.to_string()
}
