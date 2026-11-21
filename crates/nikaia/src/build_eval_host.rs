//! **What the build-time folder asks the compiler**
//! ([ADR-287](../../../docs/specification/adr/adr-287.md) D6,
//! [ADR-321](../../../docs/specification/adr/adr-321.md) D1): the folder is
//! `tools/build_eval.nika`, and it finds a `comptime`, a variant and a grammar
//! rule in the program's files itself (ADR-294, #125). This holds only what
//! Nikaia cannot hold: the names a walk above it already worked out (a
//! closure), what a build may read (`Reads`, behind a lock) and the run of a
//! grammar (`BuildTimeDir`).

#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};
use nikaia_std::compiler::build_eval::{BtHost, BtRan, BtRead, BtRefusal, bt_evaluate};
use nikaia_std::compiler::build_values::BuildValue;

use crate::assets::Reads;
use crate::ast::Expr;
use crate::contracts::ty::Ty;
use crate::parser::Parsed;

/// **What an initialiser of literals comes to**, folded while the program is
/// built: `parsed` is the file being checked, `beside` every file of the
/// program (a variant or a grammar may be declared in any of them), `reads`
/// what this build may read ([ADR-310](../../../docs/specification/adr/adr-310.md):
/// [`Reads::none`] turns the whole class off) and `known` the names already
/// worked out. A refusal is the tools' `BtRefusal` held in place, as the
/// lowered tools allow it (`result_large_err`).
#[allow(clippy::result_large_err)]
pub(crate) fn evaluate(
    expr: &Expr,
    parsed: &Parsed,
    beside: &[&Parsed],
    reads: &Reads,
    known: &dyn Fn(&str) -> Option<BuildValue>,
) -> Result<BuildValue, BtRefusal> {
    let mut units: Vec<&Parsed> = vec![parsed];
    units.extend(beside.iter().copied());
    let views: Vec<_> = units
        .iter()
        .map(|parsed| crate::check::parsed_view_of(parsed))
        .collect();
    let host = Host {
        units: &units,
        reads,
        known,
    };
    bt_evaluate(expr, &views, &host).map_err(|thrown| thrown.split().0)
}

/// Which numbers name a character: Unicode's table, handed to the Nikaia walks.
pub fn char_of(n: i64) -> Option<char> {
    u32::try_from(n).ok().and_then(char::from_u32)
}

/// The files by number (0 is the one being checked), for the run of a grammar.
struct Host<'h> {
    units: &'h [&'h Parsed],
    reads: &'h Reads,
    known: &'h dyn Fn(&str) -> Option<BuildValue>,
}

impl BtHost for Host<'_> {
    fn char_of(&self, n: i64) -> Option<char> {
        char_of(n)
    }

    fn known(&self, name: &str) -> Option<BuildValue> {
        (self.known)(name)
    }

    fn run_grammar(&self, at: i64, which: &str, rule: &str, result: &Ty, input: &str) -> BtRan {
        let ask = crate::grammar_run::Ask {
            grammar: which,
            rule,
            input,
            result,
            units: &self.units[1..],
        };
        let outcome = self
            .reads
            .build_time_dir()
            .run(self.units[at as usize], &ask)
            .and_then(|dump| crate::grammar_run::decode(&dump));
        match outcome {
            Ok(value) => BtRan {
                found: true,
                value: Some(value),
                wall: None,
            },
            Err(why) => BtRan {
                found: true,
                value: None,
                wall: Some(why),
            },
        }
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
}
