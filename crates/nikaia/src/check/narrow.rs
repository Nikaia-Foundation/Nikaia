// crates/nikaia/src/check/narrow.rs
//
// **A `T?` is a `T` where a test proves it** (ADR-350, Part I 2.3, #619):
// after `x != null`, in the rest of an `&&`, the branch the test leads into,
// a `while` body, and the rest of a block an `x == null { return }` left.
//
// A proof is a **marker binding**: a copy of the name's binding, `T?` as it
// was, pushed into a frame of the scope. A frame lives as long as its block,
// so the proof ends where the region ends. Reading the name finds the marker
// first, and the marker's key says it is proved. A write after the test
// (`written_at`), or one in a loop opened inside the region that the next
// turn reaches before the read, takes the proof back.

use super::*;
use nikaia_std::tools::check_state::CheckLabel;

/// One proof: what the name is where it is proved, and how many writes the
/// function had seen when it was made.
#[derive(Clone, Debug)]
pub(super) struct Narrow {
    pub inner: Ty,
    pub written_from: usize,
    /// The frame the marker was pushed into.
    pub frame: usize,
}

/// What a read of a name finds.
pub(super) enum Proved {
    /// Proved: the `T`.
    Yes(Ty),
    /// Proved once, but written after the test (the name and where).
    Lost(i64),
    No,
}

/// The names a condition proves not `null` where it is `holds`.
pub(super) fn proved_names(parsed: &Parsed, cond: &Expr, holds: bool) -> Vec<String> {
    match cond {
        Expr::Binary { op, lhs, rhs, .. } => match (op, holds) {
            (BinaryOp::Ne, true) | (BinaryOp::Eq, false) => match (lhs.as_ref(), rhs.as_ref()) {
                (Expr::Variable(name, _), Expr::LitNull(_))
                | (Expr::LitNull(_), Expr::Variable(name, _)) => {
                    vec![parsed.text(*name).to_string()]
                }
                _ => Vec::new(),
            },
            (BinaryOp::And, true) | (BinaryOp::Or, false) => {
                let mut names = proved_names(parsed, lhs, holds);
                names.extend(proved_names(parsed, rhs, holds));
                names
            }
            _ => Vec::new(),
        },
        Expr::Unary {
            op: UnaryOp::Not,
            expr,
            ..
        } => proved_names(parsed, expr, !holds),
        _ => Vec::new(),
    }
}

impl Checker<'_> {
    /// Where the innermost binding of `name` is: its frame and its place.
    fn bound_where(&self, name: &str) -> Option<(usize, usize, i64)> {
        for (frame, locals) in self.s.scope.iter().enumerate().rev() {
            if let Some(at) = locals.iter().rposition(|local| local.name == name) {
                return Some((frame, at, locals[at].id));
            }
        }
        None
    }

    /// The markers for `names`, each a `T?` binding proved here, for a frame
    /// that will be pushed at `frame`.
    pub(super) fn proofs_for(&mut self, names: &[String], frame: usize) -> Vec<Local> {
        let mut markers: Vec<Local> = Vec::new();
        for name in names {
            let Some(local) = self.s.binding(name) else {
                continue;
            };
            let Ty::Nullable(inner) = &local.ty else {
                continue;
            };
            if matches!(**inner, Ty::Unknown) || markers.iter().any(|m| &m.name == name) {
                continue;
            }
            let at = self.s.scope.get(frame).map_or(0, Vec::len) + markers.len();
            self.narrows.insert(
                (frame, at, local.id),
                Narrow {
                    inner: (**inner).clone(),
                    written_from: self.s.written_at.len(),
                    frame,
                },
            );
            markers.push(local.clone());
        }
        markers
    }

    /// Walk `walk` with `names` proved, in a frame of their own.
    pub(super) fn with_proved<T>(
        &mut self,
        names: &[String],
        walk: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let frame = self.s.scope.len();
        let markers = self.proofs_for(names, frame);
        if markers.is_empty() {
            return walk(self);
        }
        self.s.scope.push(markers);
        let value = walk(self);
        self.s.scope.pop();
        value
    }

    /// `names` proved for the rest of the block being walked: an `if` above
    /// left it where they were `null`.
    pub(super) fn proved_from_here(&mut self, names: &[String]) {
        let Some(frame) = self.s.scope.len().checked_sub(1) else {
            return;
        };
        let markers = self.proofs_for(names, frame);
        self.s.scope[frame].extend(markers);
    }

    /// Whether a read of `name` is proved.
    pub(super) fn proved(&self, name: &str, here: i64) -> Proved {
        let Some(key) = self.bound_where(name) else {
            return Proved::No;
        };
        let Some(narrow) = self.narrows.get(&key) else {
            return Proved::No;
        };
        if let Some((_, at)) = self.s.written_at[narrow.written_from.min(self.s.written_at.len())..]
            .iter()
            .find(|(path, at)| path == name && *at != here)
        {
            return Proved::Lost(*at);
        }
        // A loop opened inside the region that writes the name reaches this
        // read again after the write.
        if self.s.repeats.iter().any(|repeats| {
            repeats.what == "loop"
                && repeats.frame as usize > narrow.frame
                && repeats.revived.contains(name)
        }) {
            return Proved::Lost(-1);
        }
        Proved::Yes(narrow.inner.clone())
    }

    /// The reads below `expr` that ask about the `T?` itself, and are not
    /// read as the `T` a proof makes it.
    pub(super) fn reads_of_the_maybe(&mut self, expr: &Expr) {
        let asked: Vec<&Expr> = match expr {
            Expr::Coalesce { value, .. } => vec![value],
            Expr::SafeField { base, .. } => vec![base],
            Expr::SafeMethod { receiver, .. } => vec![receiver],
            Expr::Match { value, .. } => vec![value],
            Expr::Binary {
                op: BinaryOp::Eq | BinaryOp::Ne,
                lhs,
                rhs,
                ..
            } if matches!(**lhs, Expr::LitNull(_)) || matches!(**rhs, Expr::LitNull(_)) => {
                vec![lhs, rhs]
            }
            Expr::Binary { lhs, rhs, .. } => {
                for operand in [lhs, rhs] {
                    self.operand_reads.insert(value_node(operand));
                }
                return;
            }
            Expr::MethodCall { receiver, .. } | Expr::Unary { expr: receiver, .. } => {
                self.operand_reads.insert(value_node(receiver));
                return;
            }
            _ => return,
        };
        for read in asked {
            if matches!(read, Expr::Variable(..)) {
                self.maybe_reads.insert(value_node(read));
            }
        }
    }

    /// A read of a name whose binding is `ty`: the `T` where a test proves it.
    ///
    /// The written name of an assignment is not read (`reading_only`), and the
    /// assignment's own write is after the value it is given is read.
    pub(super) fn read_as_proved(&mut self, expr: &Expr, name: &str, ty: Ty, span: &Span) -> Ty {
        if !matches!(ty, Ty::Nullable(_))
            || self.s.reading_only
            || self.maybe_reads.contains(&value_node(expr))
        {
            return ty;
        }
        match self.proved(name, span.at() as i64) {
            Proved::Yes(inner) => {
                let copied = matches!(&inner, Ty::Named { name, .. } if matches!(name.as_str(),
                    "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize"
                    | "isize" | "f64" | "bool" | "char" | "scalar"));
                // **What the lowering reads as a `T` today**: a value a word
                // holds, and text. A struct or a list proved not `null` keeps
                // its `T?` below until the lowering knows each binding's Rust
                // form (owned, or a reference to the option).
                if !copied && !matches!(inner.text().as_str(), "String" | "ref String") {
                    return ty;
                }
                let operand = self.operand_reads.contains(&value_node(expr));
                self.s.facts.narrowed_reads.insert(
                    value_node(expr),
                    i64::from(copied) | (i64::from(operand) << 1),
                );
                inner
            }
            Proved::Lost(at) => {
                self.lost_reads
                    .insert(value_node(expr), (name.to_string(), at));
                ty
            }
            Proved::No => ty,
        }
    }

    /// `NK1102` over an operand whose proof a write took back: the note and
    /// the label say where.
    pub(super) fn a_proof_taken_back(&mut self, operands: [&Expr; 2], before: usize) {
        if self.s.findings.len() == before {
            return;
        }
        let Some((name, at)) = operands
            .iter()
            .find_map(|operand| self.lost_reads.get(&value_node(operand)).cloned())
        else {
            return;
        };
        let Some(finding) = self.s.findings.last_mut() else {
            return;
        };
        finding.notes.push(match at {
            -1 => format!(
                "A test before proved `{name}` is not `null`, but a loop around this use assigns `{name}`, so the next turn may read it `null`."
            ),
            _ => format!(
                "A test before proved `{name}` is not `null`, but `{name}` is assigned after the test, and from there on it is a `T?` again."
            ),
        });
        if at >= 0 {
            finding.labels.push(CheckLabel {
                span: Span {
                    start: at as u32,
                    end: at as u32 + name.len() as u32,
                },
                word: name.clone(),
                text: "assigned here, after the test".to_string(),
                main: false,
            });
        }
    }
}
