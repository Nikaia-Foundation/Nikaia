pub struct Seeds {
    tables: EmitSeedsTables,
    shapes: BTreeMap<String, Shape>,
}

/// The calls of a body (to a method, a path or a name) and the variables
/// their arguments are field chains of, for `EmitSeedsTables::hands`.
fn calls_of(parsed: &Parsed, body: &crate::ast::Block) -> Vec<EmitSeedsCall> {
    let mut out = Vec::new();
    crate::emit::visit_block(body, &mut |e| {
        let (callee, given): (String, &[crate::ast::Expr]) = match e {
            crate::ast::Expr::MethodCall { method, args, .. } => {
                (parsed.text(*method).to_string(), args)
            }
            crate::ast::Expr::Call { func, args, .. } => match &**func {
                crate::ast::Expr::Path(segments, _) => match segments.last() {
                    Some(last) => (parsed.text(*last).to_string(), args),
                    None => return,
                },
                crate::ast::Expr::Variable(one, _) => (parsed.text(*one).to_string(), args),
                _ => return,
            },
            _ => return,
        };
        out.push(EmitSeedsCall {
            callee,
            roots: given.iter().filter_map(|a| root_name(parsed, a)).collect(),
        });
    });
    out
}

/// The `impl`s without a trait of a type that declares parameters, each as
/// one body of the calls of its methods with a receiver, held in `self`.
fn impls_of(units: &[&Parsed]) -> Vec<EmitSeedsBody> {
    let mut out = Vec::new();
    for parsed in units {
        for item in &parsed.program.items {
            let Item::Impl {
                trait_name: None,
                target,
                methods,
                ..
            } = &item.node
            else {
                continue;
            };
            let mut calls = Vec::new();
            for m in methods {
                if let Item::Fn {
                    receiver: Some(_),
                    body,
                    ..
                } = &m.node
                {
                    calls.extend(calls_of(parsed, body));
                }
            }
            out.push(EmitSeedsBody {
                name: emit_seeds_last(parsed.text(target.name)),
                held: vec!["self".to_string()],
                calls,
            });
        }
    }
    out
}

impl Seeds {
    /// The traits' methods with a map position, and then the `pub` functions
    /// that hand a position they were given to one of them (or to another such
    /// function), until none is added.
    fn read_barriers(&mut self, units: &[&Parsed]) {
        for parsed in units {
            for item in &parsed.program.items {
                match &item.node {
                    Item::Trait { methods, .. } => {
                        for m in methods {
                            let held = m
                                .node
                                .args
                                .iter()
                                .map(|a| &a.ty)
                                .chain(m.node.ret_type.iter())
                                .any(|t| self.count(parsed, t) > 0);
                            if held {
                                self.tables.trait_maps = true;
                                self.tables
                                    .barriers
                                    .insert(parsed.text(m.node.name).to_string());
                            }
                        }
                    }
                    Item::Impl {
                        trait_name: Some(_),
                        methods,
                        ..
                    } => {
                        for m in methods {
                            if let Item::Fn {
                                name: Some(name),
                                args,
                                ret_type,
                                ..
                            } = &m.node
                                && args
                                    .iter()
                                    .map(|a| &a.ty)
                                    .chain(ret_type.iter())
                                    .any(|t| self.count(parsed, t) > 0)
                            {
                                self.tables.trait_maps = true;
                                self.tables.barriers.insert(parsed.text(*name).to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // **A method whose result alone holds a map position**: a method call
        // takes no turbofish here, so nothing could fill it. Its positions are
        // the keyed map, like a barrier's.
        for parsed in units {
            for item in &parsed.program.items {
                let Item::Impl {
                    trait_name: None,
                    methods,
                    ..
                } = &item.node
                else {
                    continue;
                };
                for m in methods {
                    let Item::Fn {
                        name: Some(name),
                        args,
                        ret_type: Some(ret),
                        is_public: true,
                        ..
                    } = &m.node
                    else {
                        continue;
                    };
                    let mut seen = std::collections::BTreeSet::new();
                    let given: usize = args
                        .iter()
                        .map(|a| self.distinct(parsed, &a.ty, &mut seen))
                        .sum();
                    if self.distinct(parsed, ret, &mut seen) > 0 && given == 0 {
                        self.tables.fixed.insert(parsed.text(*name).to_string());
                        self.tables.functions = true;
                    }
                }
            }
        }
        // **A private function with a map of its own in its signature** has
        // the package's class for it, not a hidden parameter: a `pub` function
        // that hands it a map it was given cannot say that the two are one
        // type, so its positions are the keyed map, as a trait method's are
        // (the package writes the keyed map in its own bodies, `seeded()`).
        self.tables.private.clear();
        for parsed in units {
            for item in &parsed.program.items {
                if let Item::Fn {
                    name: Some(name),
                    args,
                    ret_type,
                    is_public: false,
                    receiver: None,
                    ..
                } = &item.node
                    && args
                        .iter()
                        .map(|a| &a.ty)
                        .chain(ret_type.iter())
                        .any(|t| emit_seeds_bare(&parsed.interner, t) > 0)
                {
                    self.tables.private.insert(parsed.text(*name).to_string());
                }
            }
        }
        // The barrier closure is Nikaia's; here the bodies it looks at: the
        // `pub` functions, then the methods that hand a map they were given
        // (not `self`'s) on.
        let mut bodies = Vec::new();
        for parsed in units {
            for item in &parsed.program.items {
                if let Item::Fn {
                    name: Some(name),
                    args,
                    body,
                    is_public: true,
                    receiver: None,
                    ..
                } = &item.node
                {
                    self.body(&mut bodies, parsed, parsed.text(*name), args, body);
                }
            }
        }
        for parsed in units {
            for item in &parsed.program.items {
                let Item::Impl {
                    trait_name: None,
                    methods,
                    ..
                } = &item.node
                else {
                    continue;
                };
                for m in methods {
                    if let Item::Fn {
                        name: Some(name),
                        args,
                        body,
                        is_public: true,
                        receiver: Some(_),
                        ..
                    } = &m.node
                    {
                        self.body(&mut bodies, parsed, parsed.text(*name), args, body);
                    }
                }
            }
        }
        self.tables.close_barriers(&bodies);
    }

    /// A body for the barrier closure, if one of its parameters holds a map.
    fn body(
        &self,
        bodies: &mut Vec<EmitSeedsBody>,
        parsed: &Parsed,
        name: &str,
        args: &[crate::ast::FnArg],
        body: &crate::ast::Block,
    ) {
        let held: Vec<String> = args
            .iter()
            .filter(|a| self.count(parsed, &a.ty) > 0)
            .map(|a| parsed.text(a.name).to_string())
            .collect();
        if !held.is_empty() {
            bodies.push(EmitSeedsBody {
                name: name.to_string(),
                held,
                calls: calls_of(parsed, body),
            });
        }
    }


    fn count(&self, parsed: &Parsed, ty: &Type) -> usize {
        self.tables.count(&parsed.interner, ty) as usize
    }

    fn distinct(
        &self,
        parsed: &Parsed,
        ty: &Type,
        seen: &mut std::collections::BTreeSet<String>,
    ) -> usize {
        self.tables.distinct(&parsed.interner, ty, seen) as usize
    }

}
