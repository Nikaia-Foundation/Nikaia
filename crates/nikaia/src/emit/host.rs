//! **The Rust side of the emitter's core** (ADR-294, #125, #558, E9): the
//! implementation of `EmitHost` (`tools/emit_host.nika`) for the `Emitter`, and
//! what the moved methods' stubs use to cross into Nikaia.
//!
//! **The walk is a baton.** `Emitter::w` holds it as `Option<EmitWalk>`. A stub
//! takes it out (`nikaia`), passes `&mut` into the Nikaia method, and puts it
//! back; a host method does the opposite for the time it runs Rust code (it
//! swaps the walk it was handed in, and takes it out again at the end). A
//! forgotten swap is a panic at the next `walk()`, never a silent copy.
//!
//! **The flow converts only at the crossing**: `Flow::to_emit` splits it into
//! the by-value `EmitFlow` and the owned `EmitScope`, `Flow::from_emit` joins
//! them again.
//!
//! **An error keeps its identity**: a refusal raised by Rust code under a
//! crossing (`refused_at!`, an `anyhow::Error`) is stashed in `Emitter::failed`
//! and a marker error travels through the Nikaia frames; the stub that gets it
//! back returns the stash. An `EmitRefused` thrown by Nikaia code becomes the
//! same `Refused` the Rust side raises, with its position.

use anyhow::Result;
use nikaia_std::tools::emit_host::EmitHost;
use nikaia_std::tools::emit_kinds::{EmitFlow, EmitRefused, EmitScope};
use nikaia_std::tools::emit_state::EmitWalk;

use super::{Emitter, Flow, Out, Tail};
use crate::ast::{Block, Expr};

type Crossing = Result<(), Box<dyn std::error::Error>>;

impl<'a> Flow<'a> {
    /// The flow as Nikaia sees it: the flags by value, the rest owned.
    pub(super) fn to_emit(self) -> (EmitFlow, EmitScope) {
        (
            EmitFlow {
                can_throw: self.throws,
                caught: self.caught,
                caught_named: self.caught_named,
                in_a_place: self.in_a_place,
                arms_are_statements: self.arms_are_statements,
                statement: match self.statement {
                    usize::MAX => -1,
                    at => at as i64,
                },
                handled_here: self.handled_here,
                in_lambda: self.in_lambda,
                widen: self.widen,
                inferred: self.inferred,
                in_loop: self.in_loop,
            },
            EmitScope {
                origin: self.origin.to_string(),
                function: self.function.to_string(),
                channel: self.channel.to_string(),
                caught_sum: self.caught_sum.map(str::to_string),
                caught_member: self.caught_member.map(str::to_string),
                bound: self.bound.to_string(),
                changed: self.changed.to_vec(),
                awaited: self.awaited.to_vec(),
                tasks_channel: self.tasks_channel.to_string(),
                tasks_function: self.tasks_function.to_string(),
            },
        )
    }

    /// The same flow back, borrowing the scope.
    pub(super) fn from_emit(flow: EmitFlow, scope: &'a EmitScope) -> Flow<'a> {
        Flow {
            throws: flow.can_throw,
            origin: &scope.origin,
            caught: flow.caught,
            caught_named: flow.caught_named,
            caught_sum: scope.caught_sum.as_deref(),
            caught_member: scope.caught_member.as_deref(),
            in_a_place: flow.in_a_place,
            arms_are_statements: flow.arms_are_statements,
            statement: match flow.statement {
                -1 => usize::MAX,
                at => at as usize,
            },
            function: &scope.function,
            handled_here: flow.handled_here,
            channel: &scope.channel,
            in_lambda: flow.in_lambda,
            changed: &scope.changed,
            awaited: &scope.awaited,
            tasks_channel: &scope.tasks_channel,
            tasks_function: &scope.tasks_function,
            bound: &scope.bound,
            widen: flow.widen,
            inferred: flow.inferred,
            in_loop: flow.in_loop,
        }
    }
}

impl Emitter<'_> {
    /// Run `f` with the walk handed to Nikaia: the baton leaves the emitter for
    /// the time, and comes back.
    pub(super) fn nikaia<R>(&self, f: impl FnOnce(&mut EmitWalk) -> R) -> R {
        let mut walk = self
            .w
            .borrow_mut()
            .take()
            .expect("the walk is already out on a crossing");
        let result = f(&mut walk);
        *self.w.borrow_mut() = Some(walk);
        result
    }

    /// A moved arm: the flow split for the crossing, `f` run with the walk, the
    /// result as the Rust side raises it.
    pub(super) fn cross(
        &self,
        flow: Flow<'_>,
        f: impl FnOnce(&mut EmitWalk, EmitFlow, &EmitScope) -> Crossing,
    ) -> Result<()> {
        let (flow, scope) = flow.to_emit();
        let crossed = self.nikaia(|walk| f(walk, flow, &scope));
        self.crossed(crossed)
    }

    /// The result of a crossing into Nikaia, as the Rust side raises it.
    pub(super) fn crossed<T>(&self, result: Result<T, Box<dyn std::error::Error>>) -> Result<T> {
        let error = match result {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        if let Some(stashed) = self.failed.borrow_mut().take() {
            return Err(stashed);
        }
        let mut link: Option<&(dyn std::error::Error + 'static)> = Some(&*error);
        while let Some(each) = link {
            // A `throw` in Nikaia wraps the error in its envelope.
            let each = match each.downcast_ref::<nikaia_std::error::Raised>() {
                Some(raised) => raised.inner(),
                None => each,
            };
            if let Some(refused) = each.downcast_ref::<EmitRefused>() {
                return Err(match refused.at {
                    Some(at) => crate::diagnostics::refuse_at(at as usize, refused.message.clone()),
                    None => crate::diagnostics::refuse(refused.message.clone()),
                });
            }
            link = each.source();
        }
        Err(crate::diagnostics::refuse(error.to_string()))
    }

    /// An error raised by Rust code under a crossing: kept whole, with a marker
    /// in its place for the Nikaia frames to carry up.
    fn stash(&self, error: anyhow::Error) -> Box<dyn std::error::Error> {
        let message = error.to_string();
        *self.failed.borrow_mut() = Some(error);
        Box::new(EmitRefused { message, at: None })
    }

    /// Run Rust code on behalf of Nikaia: the walk it holds goes back into the
    /// baton, and comes out again at the end.
    fn hosted(&self, walk: &mut EmitWalk, run: impl FnOnce() -> Result<()>) -> Crossing {
        let held = std::mem::replace(walk, EmitWalk::fresh());
        let before = self.w.borrow_mut().replace(held);
        assert!(before.is_none(), "the walk was in two places at once");
        let result = run();
        *walk = self
            .w
            .borrow_mut()
            .take()
            .expect("the walk was taken during a hosted call");
        result.map_err(|error| self.stash(error))
    }
}

impl EmitHost for Emitter<'_> {
    fn expr(
        &self,
        walk: &mut EmitWalk,
        out: &mut Out,
        node: &Expr,
        depth: i64,
        flow: EmitFlow,
        scope: &EmitScope,
    ) -> Crossing {
        self.hosted(walk, || {
            Emitter::expr(
                self,
                out,
                node,
                depth as usize,
                Flow::from_emit(flow, scope),
            )
        })
    }

    fn block_opening_with(
        &self,
        walk: &mut EmitWalk,
        out: &mut Out,
        block: &Block,
        depth: i64,
        flow: EmitFlow,
        scope: &EmitScope,
        tail: Tail,
        opening: Option<String>,
    ) -> Crossing {
        self.hosted(walk, || {
            Emitter::block_opening_with(
                self,
                out,
                block,
                depth as usize,
                Flow::from_emit(flow, scope),
                tail,
                opening.as_deref(),
            )
        })
    }

    fn wrap_hole(&self, hole: &Expr, wraps: &[(i64, winnow_grammar::Symbol)]) -> Expr {
        let wraps: Vec<(usize, winnow_grammar::Symbol)> = wraps
            .iter()
            .map(|(at, name)| (*at as usize, *name))
            .collect();
        let mut wrapped = hole.clone();
        crate::text_tiers::wrap_hole(&mut wrapped, &wraps);
        wrapped
    }

    fn copied_bindings(&self, at: i64) -> Vec<String> {
        self.s
            .facts
            .copied_bindings
            .range((at, String::new())..)
            .take_while(|(pattern, _)| *pattern == at)
            .map(|(_, name)| name.clone())
            .collect()
    }

    fn guard_copies(&self, at: i64) -> Vec<String> {
        self.s
            .facts
            .guards_inside_boxes
            .get(&at)
            .map(|names| names.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn take_lend(&self, out: &mut Out) -> bool {
        super::OutText::take_lend(out)
    }

    fn wants_widening(&self, node: &Expr) -> bool {
        matches!(
            crate::fold::constant_of(node, &crate::fold::nothing_is_known),
            Some(folded) if crate::fold::wants_widening(&folded)
        )
    }

    fn block_mentions(&self, block: &Block, name: &str) -> bool {
        crate::contracts::order::block_mentions(self.parsed, block, name)
    }

    fn error_type(&self, node: &Expr) -> Option<String> {
        crate::contracts::throws::error_type(self.parsed, node)
    }

    fn task_handles(&self, task: i64) -> Vec<String> {
        self.s
            .facts
            .task_handles
            .iter()
            .filter(|(at, _)| *at == task)
            .map(|(_, name)| name.clone())
            .collect()
    }

    fn overlaps_user_code(&self) -> bool {
        self.build.overlaps_user_code()
    }

    fn crossings_are_possible(&self) -> bool {
        self.build.crossings_are_possible()
    }

    fn user_parallelism(&self) -> bool {
        matches!(self.build.user_parallelism, super::UserParallelism::Yes)
    }

    fn parallelism(&self) -> String {
        self.build.parallelism().to_string()
    }

    fn written(&self, expr: &Expr) -> String {
        crate::check::written(self.parsed, expr)
    }

    fn held_buffers(
        &self,
        walk: &EmitWalk,
        function: &str,
        statement: i64,
        root: &str,
    ) -> Option<Vec<(i64, Vec<i64>)>> {
        let plans = &self.s.keep_plans;
        let plan = match plans.get(function) {
            Some(plan) => plan,
            None => plans.get(walk.keep_function.as_str())?,
        };
        let by_arg = plan.holds.get(&(statement, root.to_string()))?;
        let width = plan.widths.get(root).copied().unwrap_or(1);
        Some(
            by_arg
                .iter()
                .map(|(arg, buffers)| {
                    let mut keeps: Vec<i64> = buffers.iter().copied().collect();
                    while (keeps.len() as i64) < width {
                        keeps.push(keeps[keeps.len() - 1]);
                    }
                    (*arg, keeps)
                })
                .collect(),
        )
    }

    /// Every name read anywhere inside `expr`, the statements of its blocks
    /// included (a copy an arm body reads in a statement is still read).
    fn names_read(&self, expr: &Expr) -> Vec<String> {
        use crate::contracts::sync::{
            visit_expr, visit_expr_blocks, visit_stmt, visit_stmt_blocks,
        };
        fn names_in(host: &Emitter<'_>, expr: &Expr, names: &mut Vec<String>) {
            visit_expr(host.parsed, expr, &mut |each| {
                if let Expr::Variable(name, _) = each {
                    names.push(host.text(*name).to_string());
                }
            });
            visit_expr_blocks(expr, &mut |block| names_in_block(host, block, names));
        }
        fn names_in_block(host: &Emitter<'_>, block: &crate::ast::Block, names: &mut Vec<String>) {
            for stmt in &block.stmts {
                visit_stmt(host.parsed, &stmt.node, &mut |each| {
                    if let Expr::Variable(name, _) = each {
                        names.push(host.text(*name).to_string());
                    }
                });
                visit_stmt_blocks(&stmt.node, &mut |inner| names_in_block(host, inner, names));
            }
        }
        let mut names = Vec::new();
        names_in(self, expr, &mut names);
        names
    }
}
