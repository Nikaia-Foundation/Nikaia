// crates/nikaia/src/one_variant.rs
//
// **An enum with exactly one variant is written by its type's name**
// ([ADR-346](../../../docs/specification/adr/adr-347.md)): `Exit(c)` is
// `Exit::Code(c)`, at the value and in a pattern. The short form is rewritten
// to the long one right after parsing, so the checker, every analysis and the
// lowering only ever see the long form (D1). A short form of an enum with
// several variants is `NK1252` (D3).

use std::collections::BTreeMap;

use winnow_grammar::{InternerContext, Symbol};

use crate::ast::{Expr, Item, MatchPattern};
use crate::parser::Parsed;

/// A variant: its name, and whether it holds nothing.
type Variant = (String, bool);

/// The variants of every enum a short form may name: the file's own by its
/// bare name, `std`'s by its qualified name.
struct Enums {
    own: BTreeMap<String, Vec<Variant>>,
    library: BTreeMap<String, Vec<Variant>>,
    aliases: BTreeMap<String, String>,
}

impl Enums {
    fn of(parsed: &Parsed) -> Enums {
        let mut own = BTreeMap::new();
        for item in &parsed.program.items {
            if let Item::Enum { name, variants, .. } = &item.node {
                own.insert(
                    parsed.text(*name).to_string(),
                    variants
                        .iter()
                        .map(|v| {
                            (
                                parsed.text(v.name).to_string(),
                                matches!(v.fields, crate::ast::VariantFields::Unit),
                            )
                        })
                        .collect(),
                );
            }
        }
        let library = crate::contracts::std_ledger()
            .types
            .iter()
            .filter(|(_, ty)| !ty.variants.is_empty())
            .map(|(name, ty)| {
                (
                    name.clone(),
                    ty.variants
                        .iter()
                        .map(|v| (v.name.clone(), v.holds.is_empty()))
                        .collect(),
                )
            })
            .collect();
        Enums {
            own,
            library,
            aliases: parsed.aliases.clone(),
        }
    }

    /// The variants of the enum `path` names, if it names one.
    fn named(&self, path: &[String]) -> Option<&Vec<Variant>> {
        if path.len() == 1 {
            return self.own.get(&path[0]);
        }
        let joined = nikaia_std::tools::text_tiers::unaliased_in(&self.aliases, &path.join("::"));
        let joined = joined.strip_prefix("std::").unwrap_or(&joined);
        self.library.get(joined)
    }
}

/// What a name written alone comes to.
enum Reading {
    /// An enum of one variant: the variant, and whether it holds nothing.
    One(String, bool),
    /// An enum of several: the message and the help.
    Several(String, String),
    NotAnEnum,
}

fn read(enums: &Enums, path: &[String]) -> Reading {
    let Some(variants) = enums.named(path) else {
        return Reading::NotAnEnum;
    };
    if let [(name, unit)] = variants.as_slice() {
        return Reading::One(name.clone(), *unit);
    }
    let written = path.join("::");
    let list = variants
        .iter()
        .map(|(v, _)| format!("`{written}::{v}`"))
        .collect::<Vec<_>>()
        .join(", ");
    Reading::Several(
        format!(
            "`{written}` has {} variants, so it can't be written by its type's name alone.",
            variants.len()
        ),
        format!(
            "Write the variant you mean: {list}. Only an enum with exactly one variant is written by its type's name."
        ),
    )
}

fn texts(interner: &InternerContext, path: &[Symbol]) -> Vec<String> {
    path.iter()
        .map(|s| interner.resolve(*s).to_string())
        .collect()
}

/// Rewrites every short form in the file to its long form, or refuses the
/// first one whose enum has several variants (`NK1252`).
pub(crate) fn expand(parsed: &mut Parsed) -> anyhow::Result<()> {
    let enums = Enums::of(parsed);
    let interner = parsed.interner.clone();
    let mut refused: Option<(u32, String, String)> = None;
    let mut items = std::mem::take(&mut parsed.program.items);
    for item in &mut items {
        crate::text_tiers::each_block(&mut item.node, &mut |block| {
            crate::text_tiers::visit_block_mut(
                block,
                &mut |expr| {
                    let id = crate::check::value_node(expr) as u32;
                    let mut refuse = |head: String, help: String| {
                        if refused.is_none() {
                            refused = Some((id, head, help));
                        }
                    };
                    rewrite(&enums, &interner, expr, &mut refuse);
                },
                &mut |_, _| {},
            )
        });
    }
    parsed.program.items = items;
    let Some((id, head, help)) = refused else {
        return Ok(());
    };
    let span = parsed
        .expr_spans
        .get(&id)
        .copied()
        .unwrap_or_else(|| crate::ast::Span::new(0, 0));
    let finding = crate::check::Finding {
        severity: crate::check::Severity::Error,
        span,
        code: "NK1252".to_string(),
        message: head.clone(),
        notes: Vec::new(),
        help: Some(help.clone()),
        labels: vec![crate::check::Label {
            span,
            word: String::new(),
            text: String::new(),
            main: true,
        }],
    };
    Err(crate::diagnostics::refuse_finding(
        finding,
        format!("NK1252: {head} {help}"),
    ))
}

fn rewrite(
    enums: &Enums,
    interner: &InternerContext,
    expr: &mut Expr,
    refuse: &mut dyn FnMut(String, String),
) {
    match expr {
        Expr::Call { func, .. } => {
            let path = match &**func {
                Expr::Variable(name, _) => vec![interner.resolve(*name).to_string()],
                Expr::Path(segments, _) => texts(interner, segments),
                _ => return,
            };
            match read(enums, &path) {
                Reading::One(variant, _) => {
                    let mut segments: Vec<Symbol> =
                        path.iter().map(|s| interner.intern_string(s)).collect();
                    segments.push(interner.intern_string(&variant));
                    **func = Expr::Path(segments, crate::ast::NodeId::fresh());
                }
                Reading::Several(head, help) => refuse(head, help),
                Reading::NotAnEnum => {}
            }
        }
        Expr::StructLit { name, .. } => {
            let text = interner.resolve(*name).to_string();
            let path: Vec<String> = text.split("::").map(str::to_string).collect();
            match read(enums, &path) {
                Reading::One(variant, _) => {
                    *name = interner.intern_string(&format!("{text}::{variant}"));
                }
                Reading::Several(head, help) => refuse(head, help),
                Reading::NotAnEnum => {}
            }
        }
        // A unit variant at the value: `Done` is `Done::Only`. A path with
        // `::` (`m::Done`) is the same, read by its qualified name.
        Expr::Variable(name, _) => {
            let path = vec![interner.resolve(*name).to_string()];
            if let Reading::One(variant, true) = read(enums, &path) {
                let segments = vec![*name, interner.intern_string(&variant)];
                *expr = Expr::Path(segments, crate::ast::NodeId::fresh());
            }
        }
        Expr::Path(segments, _) => {
            if let Reading::One(variant, true) = read(enums, &texts(interner, segments)) {
                segments.push(interner.intern_string(&variant));
            }
        }
        Expr::Match { arms, .. } => {
            for arm in arms {
                pattern(enums, interner, &mut arm.pattern, refuse);
            }
        }
        _ => {}
    }
}

fn pattern(
    enums: &Enums,
    interner: &InternerContext,
    pat: &mut MatchPattern,
    refuse: &mut dyn FnMut(String, String),
) {
    match pat {
        MatchPattern::Tuple { path, parts } => {
            for part in parts.iter_mut() {
                pattern(enums, interner, part, refuse);
            }
            if !path.is_empty() {
                widen(enums, interner, path, refuse);
            }
        }
        MatchPattern::Named { path, fields, .. } => {
            for field in fields.iter_mut() {
                pattern(enums, interner, &mut field.pat, refuse);
            }
            widen(enums, interner, path, refuse);
        }
        // A bare name binds (ADR-347 D2); only a path with `::` can name a
        // unit variant's enum.
        MatchPattern::Path(path) if path.len() >= 2 => {
            if let Reading::One(variant, true) = read(enums, &texts(interner, path)) {
                path.push(interner.intern_string(&variant));
            }
        }
        MatchPattern::Or(alternatives) => {
            for alternative in alternatives.iter_mut() {
                pattern(enums, interner, alternative, refuse);
            }
        }
        _ => {}
    }
}

fn widen(
    enums: &Enums,
    interner: &InternerContext,
    path: &mut Vec<Symbol>,
    refuse: &mut dyn FnMut(String, String),
) {
    match read(enums, &texts(interner, path)) {
        Reading::One(variant, _) => path.push(interner.intern_string(&variant)),
        Reading::Several(head, help) => refuse(head, help),
        Reading::NotAnEnum => {}
    }
}
