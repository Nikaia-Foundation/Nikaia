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

use super::{Emitter, Flow, Out};
use crate::ast::Expr;

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

    /// The result of a crossing into Nikaia, as the Rust side raises it.
    pub(super) fn crossed(&self, result: Crossing) -> Result<()> {
        let Err(error) = result else { return Ok(()) };
        if let Some(stashed) = self.failed.borrow_mut().take() {
            return Err(stashed);
        }
        let mut link: Option<&(dyn std::error::Error + 'static)> = Some(&*error);
        while let Some(each) = link {
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
}
