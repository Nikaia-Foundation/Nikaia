fn tests(items: &[Spanned<Item>]) -> usize {
    items.iter().filter(|item| matches!(item.node, Item::Test { .. })).count()
}
