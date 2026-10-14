fn bound(types: &[Ty]) -> usize {
    let (mut known, mut numbers): (Vec<Ty>, usize) = (Vec::new(), 0);
    for ty in types {
        let Ty::Named { name, args, .. } = ty else { continue; };
        let Ty::Nullable(inner) = ty else { return 0; };
        numbers += args.len() + name.len();
    }
    numbers
}
