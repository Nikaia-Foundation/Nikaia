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
use nikaia_std::tools::emit_kinds::EmitRefused;
use nikaia_std::tools::emit_seeds::EmitSeedsShape;
use nikaia_std::tools::emit_state::EmitWalk;

use super::{Emitter, Out};
use crate::ast::{Block, Expr};

type Crossing = Result<(), Box<dyn std::error::Error>>;

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

    fn given_up_unmuted(&self, at: i64) -> bool {
        self.give_ups.unmuted.contains(&at)
    }

    fn given_up_rebound(&self, at: i64) -> Option<bool> {
        self.give_ups.rebound.get(&at).copied()
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
        self.build.parallelism()
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

    fn seeds_shape(&self, key: &str) -> Option<EmitSeedsShape> {
        self.seeds.shape(key).map(|shape| EmitSeedsShape {
            groups: shape.groups.iter().map(|g| *g as i64).collect(),
            within: shape
                .within
                .iter()
                .map(|w| w.map_or(-1, |j| j as i64))
                .collect(),
            start: shape.start as i64,
        })
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

    fn char_literal(&self, written: &str) -> String {
        crate::check::char_literal(written)
    }
}
