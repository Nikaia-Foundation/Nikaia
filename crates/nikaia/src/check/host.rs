//! **What the checker's Nikaia half asks of the compiler** (ADR-294, #125,
//! #558, C-4): `CheckHost` (`tools/check_host.nika`) over the parsed files and
//! what the build may read. A stub in `mod.rs` makes one of these per call -
//! it holds borrows and nothing else - and hands it to the Nikaia method.

use super::*;
use nikaia_std::tools::build_eval::BtRead;
use nikaia_std::tools::check_host::{CheckBuilt, CheckComputed, CheckHost, CheckPlace};

pub(super) struct Host<'a> {
    pub(super) parsed: &'a Parsed,
    pub(super) beside: &'a [&'a Parsed],
    pub(super) here: Option<usize>,
    pub(super) reads: &'a Reads,
    pub(super) own: &'a Ledger,
    pub(super) library: &'a Ledger,
    pub(super) newly: &'a Newly,
}

impl Host<'_> {
    /// **Every file of the program** (Part I 9.1), this one among them, and
    /// where this one stands; a check handed none is this file alone.
    fn files(&self) -> (Vec<&Parsed>, usize) {
        let mut files: Vec<&Parsed> = self.beside.to_vec();
        let here = match self.here {
            Some(here) => here,
            None => {
                files.insert(0, self.parsed);
                0
            }
        };
        (files, here)
    }
}

/// What a grammar's wall is, as the report of it is given it.
pub(super) fn a_wall(why: &crate::grammar_run::Wall) -> CheckWall {
    use crate::grammar_run::Wall;
    match why {
        Wall::Refused { detail } => CheckWall::Refused {
            detail: detail.clone(),
        },
        Wall::NoCrossedForm { ty, because } => CheckWall::NoCrossedForm {
            ty: ty.clone(),
            because: because.clone(),
        },
        Wall::InsideAnother { grammar } => CheckWall::InsideAnother {
            outer: grammar.clone(),
        },
        Wall::NowhereToBuild => CheckWall::NowhereToBuild,
        Wall::DidNotBuild { detail } => CheckWall::DidNotBuild {
            detail: detail.clone(),
        },
        Wall::Unreadable { detail } => CheckWall::Unreadable {
            detail: detail.clone(),
        },
    }
}

impl CheckHost for Host<'_> {
    fn hole(&self, site: u32, text: &str) -> Option<Expr> {
        self.parsed.hole(site, text).ok()
    }

    fn kept_call(
        &self,
        name: winnow_grammar::Symbol,
        arity: i64,
    ) -> (Vec<winnow_grammar::Symbol>, Expr) {
        self.parsed.kept_function_call(name, arity as usize)
    }

    fn intern(&self, text: &str) -> winnow_grammar::Symbol {
        self.parsed.interner.intern_string(text)
    }

    fn wrap_hole(&self, hole: &Expr, wraps: &[(i64, winnow_grammar::Symbol)]) -> Expr {
        let mut hole = hole.clone();
        let wraps: Vec<(usize, winnow_grammar::Symbol)> = wraps
            .iter()
            .map(|(id, method)| (*id as usize, *method))
            .collect();
        crate::text_tiers::wrap_hole(&mut hole, &wraps);
        hole
    }

    fn literal_holes(&self, literal: &Expr) -> Vec<Expr> {
        crate::emit::literal_expressions_bound(self.parsed, literal)
            .into_iter()
            .map(|(hole, _)| hole)
            .collect()
    }

    fn evaluate(&self, value: &Expr, held: &BTreeMap<String, build_time::Value>) -> CheckBuilt {
        let known = |name: &str| -> Option<build_time::Value> { held.get(name).cloned() };
        let outcome =
            build_time::BuildTime::new(self.parsed, self.beside, self.own, self.reads, &known)
                .evaluate(value);
        match outcome {
            Ok(value) => CheckBuilt::Value(value),
            Err(build_time::Refusal::Unevaluable) => CheckBuilt::Unevaluable,
            Err(build_time::Refusal::NotAllowed { callee, because }) => {
                CheckBuilt::NotAllowed { callee, because }
            }
            Err(build_time::Refusal::TooDeep { callee }) => CheckBuilt::TooDeep { callee },
            Err(build_time::Refusal::OutOfBounds { at, len }) => CheckBuilt::OutOfBounds {
                at,
                len: len as i64,
            },
            Err(build_time::Refusal::Circular { ring }) => CheckBuilt::Circular { ring },
            Err(build_time::Refusal::NotHere { what, why, way_out }) => {
                CheckBuilt::NotHere { what, why, way_out }
            }
            Err(build_time::Refusal::MayNotRead { path, why }) => {
                CheckBuilt::MayNotRead { path, why }
            }
            Err(build_time::Refusal::PathIsComputed) => CheckBuilt::PathIsComputed,
            Err(build_time::Refusal::GrammarWall { grammar, rule, why }) => {
                CheckBuilt::GrammarWall {
                    owner: grammar,
                    rule,
                    why: a_wall(&why),
                }
            }
        }
    }

    fn compute_comptime(
        &self,
        value: &Expr,
        found: &Ty,
        bound: &str,
        known: &BTreeMap<String, build_time::Value>,
        steps: Option<u64>,
        bytes: Option<u64>,
    ) -> CheckComputed {
        let (files, here) = self.files();
        let workshop = self.reads.workshop();
        let known = |name: &str| known.get(name).cloned();
        let mut bounds = workshop.bounds();
        if let Some(steps) = steps {
            bounds.steps = steps;
        }
        if let Some(bytes) = bytes {
            bounds.bytes = bytes;
        }
        match crate::comptime_run::compute(
            &files,
            here,
            bound,
            value,
            found,
            &known,
            self.library,
            workshop,
            bounds,
            Some(self.own),
        ) {
            crate::comptime_run::Computed::Value(computed) => CheckComputed::Value(computed),
            crate::comptime_run::Computed::Forbidden { callee, because } => {
                CheckComputed::Forbidden {
                    callee,
                    because: because.to_string(),
                }
            }
            crate::comptime_run::Computed::Stopped(detail) => CheckComputed::Stopped(detail),
            crate::comptime_run::Computed::NotHere => CheckComputed::NotHere,
        }
    }

    fn somewhere(&self) -> bool {
        self.reads.workshop().somewhere()
    }

    fn decoded(&self, literal: &str) -> Option<String> {
        build_time::decoded(literal)
    }

    fn read(&self, path: &str) -> BtRead {
        match self.reads.read(path) {
            Ok(text) => BtRead {
                text: Some(text),
                denied: None,
            },
            Err(why) => BtRead {
                text: None,
                denied: Some(why),
            },
        }
    }

    fn assets_read_in(&self, value: &Expr, read: &BTreeMap<String, String>) -> Expr {
        let mut value = value.clone();
        if !read.is_empty() {
            let parsed = self.parsed;
            crate::text_tiers::visit_expr_mut(
                &mut value,
                &mut |expr: &mut Expr| {
                    if let Expr::Call { func, args, .. } = expr
                        && matches!(&**func, Expr::Variable(name, _)
                            if parsed.text(*name) == crate::assets::ASSET)
                        && let [Expr::LitStr { text, .. }] = args.as_slice()
                        && let Some(content) =
                            build_time::decoded(text).and_then(|path| read.get(&path))
                    {
                        *expr = Expr::LitStr {
                            text: build_time::written(content),
                            id: crate::ast::NodeId::fresh(),
                        };
                    }
                },
                &mut |_, _| {},
            );
        }
        value
    }

    fn value_fn(&self) -> String {
        crate::comptime_run::VALUE_FN.to_string()
    }

    fn place_of(&self, place: &str, here: i64) -> CheckPlace {
        let (files, _) = self.files();
        let here = here as usize;
        let stopped = Some(place)
            .and_then(|place| place.strip_prefix(crate::comptime_run::STOPPED_AT))
            .and_then(|rest| rest.split_once(':'))
            .and_then(|(unit, byte)| {
                Some((unit.parse::<usize>().ok()?, byte.parse::<u32>().ok()?))
            });
        let at = stopped
            .filter(|(unit, _)| *unit == here)
            .map(|(_, byte)| byte);
        // `@<file>:<byte>`: a place in this file is shown, and one in another
        // is named by the function it is in.
        let function = stopped
            .filter(|(unit, _)| *unit != here)
            .and_then(|(unit, byte)| {
                let file = files.get(unit)?;
                file.program.items.iter().find_map(|item| match &item.node {
                    Item::Fn {
                        name: Some(name), ..
                    } if (item.span.start..item.span.end).contains(&byte) => {
                        Some(file.text(*name).to_string())
                    }
                    _ => None,
                })
            });
        CheckPlace {
            at: at.map_or(-1, i64::from),
            function,
        }
    }

    fn written(&self, value: &Expr) -> String {
        super::written_by(self.parsed, value)
    }

    fn holes_read(&self, literal: &Expr) -> Vec<(Expr, Vec<(String, String)>)> {
        match literal {
            Expr::LitInterpolated { parts, .. } => crate::emit::interpolated_holes(parts)
                .into_iter()
                .map(|(at, hole)| {
                    (
                        crate::emit::hole_as_read(self.parsed, at, hole).into_owned(),
                        Vec::new(),
                    )
                })
                .collect(),
            _ => crate::emit::literal_expressions_bound(self.parsed, literal),
        }
    }

    fn newly_throws(&self, key: &str) -> Option<Vec<String>> {
        self.newly.throws.get(key).cloned()
    }

    /// Code below the core never walks an expression: the hubs are the core's
    /// host's (`core::Core`), until the dispatcher moves (U-c7, U-c11).
    fn declared(&self, _state: &mut CheckState, _ty: &crate::ast::Type, _span: &Span) -> Ty {
        unreachable!("`declared` is asked of the core's host, not of the services")
    }

    fn payload_of(&self, ty: &str, variant: &str) -> Option<Vec<Ty>> {
        super::payload_of(self.parsed, self.beside, ty, variant)
    }

    fn float_text(&self, number: f64) -> String {
        format!("{number:?}")
    }

    fn an_attribute_read(
        &self,
        _state: &mut CheckState,
        _args: &[Expr],
        _span: &Span,
    ) -> Option<String> {
        unreachable!("`an_attribute_read` is asked of the core's host, not of the services")
    }

    #[allow(clippy::too_many_arguments)]
    fn arguments(
        &self,
        _state: &mut CheckState,
        _key: &str,
        _written: &str,
        _contract: &FnContract,
        _given: &[Expr],
        _found: &[Ty],
        _passed: &[(String, Ty)],
        _span: &Span,
    ) -> Ty {
        unreachable!("`arguments` is asked of the core's host, not of the services")
    }

    fn a_call_that_hands_its_number_back(
        &self,
        _state: &mut CheckState,
        _contract: &FnContract,
        _bound: &BTreeMap<String, Ty>,
        _args: &[Expr],
    ) {
        unreachable!("`a_call_that_hands_its_number_back` is asked of the core's host")
    }

    fn a_standard_bound_not_met(
        &self,
        _state: &mut CheckState,
        _key: &str,
        _bound: &BTreeMap<String, Ty>,
        _span: &Span,
    ) {
        unreachable!("`a_standard_bound_not_met` is asked of the core's host")
    }

    fn a_scope_that_pauses(
        &self,
        _state: &mut CheckState,
        _args: &[Expr],
        _paused: &[i64],
        _span: &Span,
    ) {
        unreachable!("`a_scope_that_pauses` is asked of the core's host")
    }

    #[allow(clippy::too_many_arguments)]
    fn a_lambda_that_pauses_at_an_entry(
        &self,
        _state: &mut CheckState,
        _key: &str,
        _receiver: &Expr,
        _on: &Ty,
        _args: &[Expr],
        _lambda_at: i64,
        _result: Ty,
        _span: &Span,
    ) -> Ty {
        unreachable!("`a_lambda_that_pauses_at_an_entry` is asked of the core's host")
    }

    fn a_task_of_a_scope(&self, _state: &mut CheckState, _args: &[Expr]) {
        unreachable!("`a_task_of_a_scope` is asked of the core's host")
    }

    fn expr(&self, _state: &mut CheckState, _node: &Expr, _span: &Span) -> Ty {
        unreachable!("the hub `expr` is asked of the core's host, not of the services")
    }

    fn block(&self, _state: &mut CheckState, _block: &Block) -> Ty {
        unreachable!("the hub `block` is asked of the core's host, not of the services")
    }
}
