fn facade(items: &[Item]) {
    for it in items {
        match it {
            Item::Fn { name: Some(name), .. } | Item::Const { name: Some(name), .. } => {
                println!("{}", name);
            }
            _ => continue,
        }
    }
}
fn nowild(it: &Item) -> i64 {
    match it {
        Item::Fn { name: Some(name), args } => name.len() + args,
        Item::Other => 0,
    }
}
