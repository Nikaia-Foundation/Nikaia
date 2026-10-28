fn bodies(methods: &[Spanned<Item>], out: &mut Vec<Block>) {
    for method in methods {
        if let Item::Fn { body, .. } = &method.node {
            out.push(body.clone());
        }
    }
}
