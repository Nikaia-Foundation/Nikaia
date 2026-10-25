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
use nikaia_std::tools::emit_function::{EmitSeedsBegin, EmitSeedsEnd, EmitSplit};
use nikaia_std::tools::emit_host::EmitHost;
use nikaia_std::tools::emit_kinds::{EmitFlow, EmitRefused, EmitScope};
use nikaia_std::tools::emit_state::{EmitPair, EmitWalk};

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
                breaks_to: self.breaks_to,
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
            breaks_to: flow.breaks_to,
        }
    }
}

impl Emitter<'_> {
    /// Run `f` with the walk handed to Nikaia: the baton leaves the emitter for
    /// the time, and comes back.
    #[track_caller]
    pub(super) fn nikaia<R>(&self, f: impl FnOnce(&mut EmitWalk) -> R) -> R {
        let slot = self.w.borrow_mut().take();
        let mut walk = crate::baton::out_of(slot, "the walk");
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

    fn block_plain(
        &self,
        walk: &mut EmitWalk,
        out: &mut Out,
        block: &Block,
        depth: i64,
        flow: EmitFlow,
        scope: &EmitScope,
        tail: Tail,
    ) -> Crossing {
        self.hosted(walk, || {
            Emitter::block_plain(
                self,
                out,
                block,
                depth as usize,
                Flow::from_emit(flow, scope),
                tail,
                None,
            )
        })
    }

    fn known_nonnegative(&self, body: &Block) -> Option<Vec<String>> {
        self.nonnegative_bodies
            .get(&(std::ptr::from_ref(body) as usize))
            .map(|known| known.iter().map(|s| self.text(*s).to_string()).collect())
    }

    fn insert_after_first_brace(&self, out: &mut Out, text: &str) {
        super::OutText::insert_after_first_brace(out, text);
    }

    fn insert_before_last_brace(&self, out: &mut Out, text: &str) {
        super::OutText::insert_before_last_brace(out, text);
    }

    fn ty_named(
        &self,
        walk: &mut EmitWalk,
        ty: &crate::ast::Type,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut written = String::new();
        self.hosted(walk, || {
            written = self.ty_filled(ty, super::Lifetimes::Named);
            Ok(())
        })?;
        Ok(written)
    }

    fn stmt(
        &self,
        walk: &mut EmitWalk,
        out: &mut Out,
        stmt: &crate::ast::Stmt,
        span: &crate::ast::Span,
        depth: i64,
        tail: Tail,
        flow: EmitFlow,
        scope: &EmitScope,
    ) -> Crossing {
        self.hosted(walk, || {
            Emitter::stmt(
                self,
                out,
                stmt,
                span,
                depth as usize,
                tail,
                Flow::from_emit(flow, scope),
            )
        })
    }

    fn a_stmt(
        &self,
        walk: &mut EmitWalk,
        out: &mut Out,
        stmt: &crate::ast::Stmt,
        span: &crate::ast::Span,
        depth: i64,
        tail: Tail,
        flow: EmitFlow,
        scope: &EmitScope,
    ) -> Crossing {
        self.hosted(walk, || {
            Emitter::a_stmt(
                self,
                out,
                stmt,
                span,
                depth as usize,
                tail,
                Flow::from_emit(flow, scope),
            )
        })
    }

    fn order_of(&self, block: &Block) -> Vec<i64> {
        nikaia_std::tools::emit_order::emit_order(block, &self.order_proofs, &self.parsed.interner)
    }

    fn ty_of_name(
        &self,
        walk: &mut EmitWalk,
        name: winnow_grammar::Symbol,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let ty = crate::ast::Type {
            name,
            generics: Vec::new(),
            is_view: false,
            is_tuple: false,
            is_nullable: false,
            code: Box::new(None),
            count: None,
            is_mut: false,
            is_slice: false,
            either: false,
            lasting: false,
        };
        self.ty_named(walk, &ty)
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

    fn seeds_begin(
        &self,
        written: &crate::ast::Item,
        owner_target: Option<String>,
        owner_declared: bool,
    ) -> EmitSeedsBegin {
        let crate::ast::Item::Fn {
            name,
            args,
            ret_type,
            is_public,
            ..
        } = written
        else {
            return EmitSeedsBegin {
                base: 0,
                seeded: false,
                in_seeded_impl: false,
            };
        };
        let in_seeded_impl = self.place.borrow().within();
        let base = match (&owner_target, in_seeded_impl) {
            (Some(o), true) => self.seeds.arity(o),
            _ => 0,
        };
        // **A signature of a barrier** (ADR-010 D9).
        let barrier = (owner_target.is_some() && owner_declared)
            || (!owner_declared && name.is_some_and(|n| self.seeds.is_fixed(self.text(n))));
        self.floor.set(barrier);
        let seeded = !self.seeds.is_empty()
            && !barrier
            && name.is_some()
            && !owner_declared
            && args
                .iter()
                .map(|a| &a.ty)
                .chain(ret_type.iter())
                .any(|t| match *is_public {
                    true => self.seeds.positions(self.parsed, t) > 0,
                    false => self.seeds.mentions_a_struct(self.parsed, t),
                });
        if seeded {
            let shape = match (*is_public, name) {
                (true, Some(n)) => {
                    let key = match &owner_target {
                        Some(o) => format!("{}::{}", o, self.text(*n)),
                        None => self.text(*n).to_string(),
                    };
                    self.seeds.shape(&key).cloned()
                }
                _ => None,
            };
            match shape {
                Some(shape) => self.place.borrow_mut().declare_fn(base, shape),
                None => self.place.borrow_mut().declare(base),
            }
        }
        EmitSeedsBegin {
            base: base as i64,
            seeded,
            in_seeded_impl,
        }
    }

    fn seeds_end(&self, seeded: bool, base: i64) -> EmitSeedsEnd {
        self.floor.set(false);
        let base = base as usize;
        let hidden = match seeded {
            true => self.place.borrow_mut().finish(base),
            false => 0,
        };
        EmitSeedsEnd {
            hidden: hidden as i64,
            declared: super::seeds::Seeds::declared(base, hidden, false),
        }
    }

    fn seeded(&self, written: String) -> String {
        Emitter::seeded(self, written)
    }

    fn split_lifetimes(
        &self,
        ty: &crate::ast::Type,
        returns: &str,
        sources: &[String],
    ) -> Option<EmitSplit> {
        let (own, fields) = Emitter::split_lifetimes(self, Some(ty), Some(returns), sources)?;
        Some(EmitSplit {
            own: own
                .into_iter()
                .map(|(first, second)| EmitPair { first, second })
                .collect(),
            fields,
        })
    }

    fn inline_written(&self, attributes: &[crate::ast::Attribute]) -> Option<String> {
        crate::check::attributes::inline_written(self.parsed, attributes).map(str::to_string)
    }

    fn set_main_params(&self, params: &[String]) {
        *self.main_params.borrow_mut() = params.to_vec();
    }

    fn main_params(&self) -> Vec<String> {
        self.main_params.borrow().clone()
    }

    fn refuse_unwritten_item(&self, written: &crate::ast::Spanned<crate::ast::Item>) -> Crossing {
        use crate::ast::Item;
        match &written.node {
            Item::Grammar(_)
            | Item::Enum { .. }
            | Item::Struct { .. }
            | Item::Fn { .. }
            | Item::Impl { .. }
            | Item::Trait { .. }
            | Item::Import { .. }
            | Item::Comptime { .. }
            | Item::Extern { .. } => Ok(()),
            other => Err(self.stash(crate::diagnostics::refuse_at(
                written.span.at(),
                format!("The compiler can't translate this item yet: {other:?}"),
            ))),
        }
    }

    fn counts_steps(&self) -> bool {
        self.build.counts_steps
    }

    fn skip_free_at_exit(&self) -> bool {
        self.build.skip_free_at_exit
    }

    fn seeds_arity(&self, name: &str) -> i64 {
        self.seeds.arity(name) as i64
    }

    fn place_declare_type(&self, base: i64) {
        self.place.borrow_mut().declare_type(base as usize);
    }

    fn place_finish(&self, base: i64) -> i64 {
        self.place.borrow_mut().finish(base as usize) as i64
    }

    fn place_enter(&self, target: &str, params: &[String]) {
        self.place.borrow_mut().enter(target, params.to_vec());
    }

    fn place_leave(&self) {
        self.place.borrow_mut().leave();
    }

    fn set_floor(&self, on: bool) {
        self.floor.set(on);
    }

    fn char_literal(&self, written: &str) -> String {
        crate::check::char_literal(written)
    }
}
