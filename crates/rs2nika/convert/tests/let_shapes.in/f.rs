fn pick(e: &Expr) -> usize {
    let (callee, given): (String, &[Expr]) = match e {
        Expr::Call { name, args } => (name.clone(), args),
        _ => return 0,
    };
    given.len()
}

fn first(items: &[Item]) -> usize {
    for m in items {
        let Item::Fn { name: Some(name), args, ret_type: Some(ret), .. } = &m.node else {
            continue;
        };
        return args.len();
    }
    0
}

fn named(bodies: &mut Vec<Body>, name: &str) {
    bodies.push(Body { name: name.to_string() });
}
