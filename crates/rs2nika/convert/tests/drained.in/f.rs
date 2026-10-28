fn as_functions(items: &mut Vec<Spanned<Item>>) {
    for item in &mut items {
        let Item::Test { name, body } = &item.node else {
            continue;
        };
        item.node = Item::Fn {
            name: name.clone(),
            body: body.clone(),
        };
    }
}
