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

/// The name a call run for a claim is computed under: none a program can
/// spell, so no `comptime` of the program is left out for it.
const CLAIM_BOUND: &str = "__nikaia_claim_value";

impl Host<'_> {
    /// **A call an open claim reads, run as a `comptime` is**
    /// ([ADR-359](../../../../docs/specification/adr/adr-359.md) D11, D12):
    /// compiled with the program's own lowering and run under ADR-321's
    /// bounds, never interpreted. `call` is the call as written, `callee` its
    /// ledger key and `args` its arguments' values; `comptimes` the program's
    /// integer `comptime`s, which the callee may read.
    ///
    /// **Only a `sync` callee that touches nothing runs**: `args read`, which
    /// a `comptime` may do, is the builder's arguments at build time and the
    /// user's when the program runs, so its value would not be the program's.
    pub(super) fn run_for_a_claim(
        &self,
        call: &Expr,
        callee: &str,
        args: &[i64],
        comptimes: &BTreeMap<String, i64>,
    ) -> crate::prove::ClaimRun {
        use crate::prove::ClaimRun;
        use nikaia_std::tools::integers::{integer_as_count, integer_of_count};
        let not_run = |why: &str| ClaimRun::NotRun(why.to_string());
        if !self.somewhere() {
            return not_run("This build has no place to compile build-time code in.");
        }
        let Some(contract) = self.own.functions.get(callee) else {
            return not_run("It is not a function of this package.");
        };
        if let Some(because) = build_time::may_not_run(contract) {
            return not_run(because);
        }
        if !contract.touches.is_empty() {
            return not_run(
                "It reads the program's arguments, which while the program is built are the \
                 builder's, not the user's.",
            );
        }
        if !contract.fails_with.is_empty() {
            return not_run("It can throw, and a claim reads only a value it hands back.");
        }
        let Some(result) = contract
            .signature
            .as_ref()
            .and_then(|signature| signature.result.clone())
        else {
            return not_run("Nothing says what it hands back.");
        };
        let Expr::Call { func, .. } = call else {
            return not_run("It is not a call of a function by name.");
        };
        let func = match &**func {
            Expr::Variable(name, _) => Expr::Variable(*name, crate::ast::NodeId::fresh()),
            Expr::Path(segments, _) => Expr::Path(segments.clone(), crate::ast::NodeId::fresh()),
            _ => return not_run("It is not a call of a function by name."),
        };
        let value = Expr::Call {
            func: Box::new(func),
            args: args
                .iter()
                .map(|value| crate::ast::int_literal(i128::from(*value)))
                .collect(),
            config: Vec::new(),
            id: crate::ast::NodeId::fresh(),
        };
        let known: BTreeMap<String, build_time::Value> = comptimes
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    build_time::Value::Int(integer_of_count(*value)),
                )
            })
            .collect();
        match self.compute_comptime(&value, &result, CLAIM_BOUND, &known, None, None) {
            CheckComputed::Value(build_time::Value::Int(n)) => match integer_as_count(&n) {
                Some(value) => ClaimRun::Value(value),
                None => not_run("Its value is more than a whole number of the proof holds."),
            },
            CheckComputed::Value(_) => not_run("It did not hand back a whole number."),
            CheckComputed::Forbidden { callee, because } => ClaimRun::NotRun(format!(
                "`{callee}` may not run while the program is built: {because}"
            )),
            CheckComputed::Stopped(detail) => stopped_as_said(&detail),
            CheckComputed::NotHere(_) => {
                not_run("This compiler can't compile it while the program is built.")
            }
        }
    }
}

/// **A run that stopped, in the words the program would print**
/// (ADR-321 D16, D17), or a bound of the build it went past (D7, D10), which
/// is no stop of the program: the claim stays a check.
fn stopped_as_said(detail: &str) -> crate::prove::ClaimRun {
    use crate::prove::ClaimRun;
    if let Some(over) = detail
        .lines()
        .find_map(|line| line.strip_prefix("nikaia-build-time: "))
    {
        return ClaimRun::NotRun(format!(
            "It went past what a build-time run may spend: {}",
            over.trim()
        ));
    }
    let line = detail
        .lines()
        .find(|line| line.contains("the program stopped"))
        .unwrap_or(detail);
    let said = line
        .split_once("the program stopped: ")
        .map(|(_, said)| said)
        .unwrap_or(line);
    ClaimRun::Stopped(said.trim().trim_end_matches('.').to_string())
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
        func: &Expr,
        arity: i64,
        receiver: bool,
    ) -> (Vec<winnow_grammar::Symbol>, Expr) {
        self.parsed
            .kept_function_call(func, arity as usize, receiver)
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

    fn free_calls(&self, body: &crate::ast::Block) -> BTreeSet<String> {
        crate::contracts::locks::free_calls(self.parsed, body)
    }

    fn literal_holes(&self, literal: &Expr) -> Vec<Expr> {
        crate::emit::literal_expressions_bound(self.parsed, literal)
            .into_iter()
            .map(|(hole, _)| hole)
            .collect()
    }

    fn evaluate(&self, value: &Expr, held: &BTreeMap<String, build_time::Value>) -> CheckBuilt {
        let known = |name: &str| -> Option<build_time::Value> { held.get(name).cloned() };
        let outcome = build_time::BuildTime::new(self.parsed, self.beside, self.reads, &known)
            .evaluate(value);
        match outcome {
            Ok(value) => CheckBuilt::Value(value),
            Err(build_time::Refusal::Unevaluable) => CheckBuilt::Unevaluable,
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
            crate::comptime_run::Computed::NotHere(why) => CheckComputed::NotHere(why),
        }
    }

    fn somewhere(&self) -> bool {
        self.reads.workshop().somewhere()
    }

    fn decoded(&self, literal: &str) -> Option<String> {
        build_time::decoded(literal)
    }

    fn composed(&self, text: &str) -> String {
        nikaia_std::text::normalized(text, nikaia_std::text::Form::Nfc)
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

    fn type_of(&self, ty: &crate::ast::Type) -> Ty {
        Ty::from_ast(self.parsed, ty)
    }

    fn constants_beside(&self) -> Vec<(String, Ty)> {
        let mut found = Vec::new();
        for (at, other) in self.beside.iter().enumerate() {
            if Some(at) == self.here {
                continue;
            }
            for item in &other.program.items {
                let Item::Comptime { name, ty, .. } = &item.node else {
                    continue;
                };
                let declared = ty
                    .as_ref()
                    .map(|ty| Ty::from_ast(other, ty))
                    .unwrap_or(Ty::Unknown);
                found.push((other.text(*name).to_string(), declared));
            }
        }
        found
    }

    fn an_escape_nothing_names(
        &self,
        text: &str,
    ) -> Option<nikaia_std::tools::escapes::NotAnEscape> {
        crate::build_time::an_escape_nothing_names(text)
    }
}
