fn args_of(t: &Ty) -> Option<Vec<Ty>> {
    match t {
        Ty::Named { args, .. } => Some(args.clone()),
        _ => None,
    }
}

fn first_arg(t: &Ty) -> Option<Ty> {
    if t.args.is_empty() {
        return None;
    }
    Some(t.args[0].clone())
}
